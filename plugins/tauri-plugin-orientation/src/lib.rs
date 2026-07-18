use serde::{Deserialize, Serialize};
use tauri::{plugin::TauriPlugin, Runtime};

#[cfg(target_os = "android")]
use tauri::Manager;
#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(target_os = "android")]
    #[error(transparent)]
    MobilePlugin(#[from] tauri::plugin::mobile::PluginInvokeError),
    #[error("orientation control is only available on Android")]
    Unsupported,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOrientationRequest {
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrientationResponse {
    pub mode: String,
    pub orientation: String,
}

#[cfg(target_os = "android")]
pub struct Orientation<R: Runtime>(PluginHandle<R>);

#[tauri::command]
async fn set_orientation<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: SetOrientationRequest,
) -> Result<OrientationResponse, Error> {
    #[cfg(target_os = "android")]
    {
        return app
            .state::<Orientation<R>>()
            .0
            .run_mobile_plugin("setOrientation", request)
            .map_err(Into::into);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (app, request);
        Err(Error::Unsupported)
    }
}

#[tauri::command]
async fn get_orientation<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<OrientationResponse, Error> {
    #[cfg(target_os = "android")]
    {
        return app
            .state::<Orientation<R>>()
            .0
            .run_mobile_plugin("getOrientation", ())
            .map_err(Into::into);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err(Error::Unsupported)
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("orientation")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin("com.moeplay.orientation", "OrientationPlugin")?;
                app.manage(Orientation(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                // Desktop has no orientation bridge. Keep the plugin registered
                // so the frontend gets a stable, explicit Unsupported response.
                let _ = (app, api);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![set_orientation, get_orientation])
        .build()
}
