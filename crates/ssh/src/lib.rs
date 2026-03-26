rust_i18n::i18n!("locales", fallback = "en");

mod ssh;

pub use ssh::{
    AuthFailureMessages, ChannelEvent, JumpServerConnectConfig, LocalPortForwardTunnel,
    ProxyConnectConfig, ProxyType, PtyConfig, RusshChannel, RusshClient, SshAuth, SshChannel,
    SshClient, SshConnectConfig, authenticate_session, authenticate_session_with_fallbacks,
    authenticate_with_strategy, expand_auto_publickey_auth, start_local_port_forward,
};
