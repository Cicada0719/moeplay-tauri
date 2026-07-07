// 萌游 MoeGame · 文件系统安全作用域
//
// 用于验证用户传入/归档解压等路径是否位于允许的前缀范围内，防止目录遍历。

use std::path::{Component, Path, PathBuf};

/// 文件系统安全作用域。
///
/// 维护一组允许的目录前缀，并提供 `resolve` 校验：
/// - 拒绝包含 `..` 的路径片段；
/// - 拒绝不在任何允许前缀下的绝对路径；
/// - 拒绝父目录不存在的新文件路径；
/// - 返回规范化（canonical）后的绝对路径。
#[derive(Debug, Clone)]
pub struct SecurityScope {
    allowed_prefixes: Vec<PathBuf>,
}

impl SecurityScope {
    /// 创建一个空的作用域。
    pub fn new() -> Self {
        Self {
            allowed_prefixes: Vec::new(),
        }
    }

    /// 添加一个允许的前缀目录。
    ///
    /// 若目录已存在，会尽量规范化为绝对路径，以便后续 containment 检查。
    pub fn allow<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let path = normalize_path(path.as_ref());
        let canon = if path.exists() {
            std::fs::canonicalize(&path).unwrap_or(path)
        } else if let Some(parent) = path.parent() {
            // 即使前缀目录本身尚未创建，也尝试规范其父目录，
            // 避免 Windows 上普通路径与 UNC 路径组件不一致的问题。
            if parent.exists() {
                std::fs::canonicalize(parent)
                    .map(|p| p.join(path.file_name().unwrap_or_default()))
                    .unwrap_or(path)
            } else {
                path
            }
        } else {
            path
        };
        self.allowed_prefixes.push(canon);
        self
    }

    /// 校验输入路径是否位于允许的作用域内，并返回规范化后的路径。
    pub fn resolve<P: AsRef<Path>>(&self, input: P) -> Result<PathBuf, String> {
        let input = normalize_path(input.as_ref());

        if has_parent_component(&input) {
            return Err("路径包含非法的 .. 片段".to_string());
        }

        let candidate = if input.is_absolute() {
            input
        } else {
            let prefix = self
                .allowed_prefixes
                .first()
                .ok_or_else(|| "没有配置允许的路径前缀".to_string())?;
            prefix.join(input)
        };

        let parent = candidate
            .parent()
            .ok_or_else(|| "无法获取父目录".to_string())?;
        if !parent.exists() {
            return Err("父目录不存在".to_string());
        }

        let canon_parent =
            std::fs::canonicalize(parent).map_err(|e| format!("规范化父目录失败: {}", e))?;

        if !self.is_under_any_prefix(&canon_parent) {
            return Err("路径不在允许范围内".to_string());
        }

        if candidate.exists() {
            std::fs::canonicalize(&candidate).map_err(|e| format!("规范化路径失败: {}", e))
        } else {
            let file_name = candidate
                .file_name()
                .ok_or_else(|| "无法获取文件名".to_string())?;
            Ok(canon_parent.join(file_name))
        }
    }

    fn is_under_any_prefix(&self, path: &Path) -> bool {
        self.allowed_prefixes
            .iter()
            .any(|prefix| is_path_under_prefix(path, prefix))
    }
}

impl Default for SecurityScope {
    fn default() -> Self {
        Self::new()
    }
}

/// 便捷构造：包含应用数据目录与用户下载目录的作用域。
///
/// 注意：`dirs` crate 5.x 未暴露 `app_data_dir()`，Windows 上 `data_dir()`
/// 已对应 Roaming AppData，因此这里不再额外处理。
pub fn app_data_scope() -> Result<SecurityScope, String> {
    let mut scope = SecurityScope::new();

    if let Some(d) = dirs::data_dir() {
        scope.allow(d.join("moeplay"));
    } else {
        return Err("无法获取用户数据目录".to_string());
    }

    if let Some(d) = dirs::data_local_dir() {
        scope.allow(d.join("moeplay"));
    }

    if let Some(d) = dirs::download_dir() {
        scope.allow(d);
    }

    if scope.allowed_prefixes.is_empty() {
        return Err("无法初始化安全作用域".to_string());
    }

    Ok(scope)
}

fn normalize_path(path: &Path) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.to_string_lossy().replace('/', "\\"))
    } else {
        path.to_path_buf()
    }
}

fn has_parent_component(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

fn is_path_under_prefix(child: &Path, prefix: &Path) -> bool {
    let prefix_components: Vec<Component<'_>> = prefix.components().collect();
    let child_components: Vec<Component<'_>> = child.components().collect();

    if child_components.len() < prefix_components.len() {
        return false;
    }

    for i in 0..prefix_components.len() {
        if cfg!(windows) {
            let a = prefix_components[i]
                .as_os_str()
                .to_string_lossy()
                .to_lowercase();
            let b = child_components[i]
                .as_os_str()
                .to_string_lossy()
                .to_lowercase();
            if a != b {
                return false;
            }
        } else if prefix_components[i] != child_components[i] {
            return false;
        }
    }

    true
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_rejects_traversal() {
        let tmp = std::env::temp_dir().join("moe_security_traversal");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut scope = SecurityScope::new();
        scope.allow(&tmp);

        let evil = tmp.join("..").join("evil.txt");
        assert!(scope.resolve(&evil).is_err());

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_allows_allowed_path() {
        let tmp = std::env::temp_dir().join("moe_security_allowed");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("file.txt"), b"ok").unwrap();

        let mut scope = SecurityScope::new();
        scope.allow(&tmp);

        let resolved = scope.resolve(tmp.join("file.txt"));
        assert!(resolved.is_ok());
        assert!(resolved.unwrap().ends_with("file.txt"));

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_rejects_nonexistent_parent() {
        let tmp = std::env::temp_dir().join("moe_security_parent");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut scope = SecurityScope::new();
        scope.allow(&tmp);

        let bad = tmp.join("not_exist").join("file.txt");
        assert!(scope.resolve(&bad).is_err());

        fs::remove_dir_all(&tmp).ok();
    }
}
