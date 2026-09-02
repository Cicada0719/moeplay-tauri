import java.io.File
import org.apache.tools.ant.taskdefs.condition.Os
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.logging.LogLevel
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.TaskAction

open class BuildTask : DefaultTask() {
    @Input
    var rootDirRel: String? = null
    @Input
    var target: String? = null
    @Input
    var release: Boolean? = null

    @TaskAction
    fun assemble() {
        val executable = resolveNodeExecutable()
        try {
            runTauriCli(executable)
        } catch (e: Exception) {
            if (Os.isFamily(Os.FAMILY_WINDOWS)) {
                // Try different Windows-specific extensions
                val fallbacks = listOf(
                    "$executable.exe",
                    "$executable.cmd",
                    "$executable.bat",
                )
                
                var lastException: Exception = e
                for (fallback in fallbacks) {
                    try {
                        runTauriCli(fallback)
                        return
                    } catch (fallbackException: Exception) {
                        lastException = fallbackException
                    }
                }
                throw lastException
            } else {
                throw e;
            }
        }
    }

    private fun resolveNodeExecutable(): String {
        val configured = System.getenv("MOEPLAY_NODE")?.trim()
        if (!configured.isNullOrEmpty()) {
            return configured
        }
        return if (Os.isFamily(Os.FAMILY_WINDOWS)) "node.exe" else "node"
    }

    fun runTauriCli(executable: String) {
        val rootDirRel = rootDirRel ?: throw GradleException("rootDirRel cannot be null")
        val target = target ?: throw GradleException("target cannot be null")
        val release = release ?: throw GradleException("release cannot be null")
        val rustRoot = File(project.projectDir, rootDirRel).canonicalFile
        val workspaceRoot = rustRoot.parentFile
            ?: throw GradleException("Unable to resolve workspace root from rootDirRel")
        val tauriCli = File(workspaceRoot, "node_modules/@tauri-apps/cli/tauri.js")
        if (!tauriCli.isFile) {
            throw GradleException(
                "Tauri CLI not found at ${tauriCli.absolutePath}; install the project npm dependencies first"
            )
        }
        val args = listOf(tauriCli.absolutePath, "android", "android-studio-script")

        project.exec {
            workingDir(rustRoot)
            executable(executable)
            args(args)
            if (project.logger.isEnabled(LogLevel.DEBUG)) {
                args("-vv")
            } else if (project.logger.isEnabled(LogLevel.INFO)) {
                args("-v")
            }
            if (release) {
                args("--release")
            }
            args(listOf("--target", target))
        }.assertNormalExitValue()
    }
}
