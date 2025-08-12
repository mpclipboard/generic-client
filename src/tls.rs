use anyhow::{Context as _, Result, anyhow};
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt as _;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tokio_rustls::TlsConnector;

pub struct TLS;

static TLS_CONNECTOR: OnceCell<TlsConnector> = OnceCell::const_new();

impl TLS {
    pub fn init() -> Result<()> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let config = ClientConfig::with_platform_verifier()
            .context("failed to create SSL client with platform verifier")?;
        let connector = TlsConnector::from(Arc::new(config));

        TLS_CONNECTOR
            .set(connector)
            .map_err(|_| anyhow!("websocket::init_connector must be called exactly once"))?;

        Ok(())
    }

    pub(crate) fn get() -> Result<TlsConnector> {
        TLS_CONNECTOR
            .get()
            .context("TLS connector is not set, did you call TLS::init() ?")
            .cloned()
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_setup_rustls_on_jvm(
    env: *mut jni::sys::JNIEnv,
    context: jni::sys::jobject,
) {
    let mut env = match unsafe { jni::JNIEnv::from_raw(env) } {
        Ok(env) => env,
        Err(err) => {
            log::error!("JNIEnv::from_raw failed: {:?}", err);
            return;
        }
    };
    let context = unsafe { jni::objects::JObject::from_raw(context) };

    if let Err(err) = rustls_platform_verifier::android::init_hosted(&mut env, context) {
        log::error!("Failed to instantiate rustls_platform_verifier: {err:?}");
    }
}
