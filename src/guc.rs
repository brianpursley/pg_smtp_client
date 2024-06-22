use core::ffi::CStr;
use pgrx::*;

pub static SMTP_SERVER: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);
pub static SMTP_PORT: GucSetting<i32> = GucSetting::<i32>::new(587);
pub static SMTP_TLS: GucSetting<bool> = GucSetting::<bool>::new(true);
pub static SMTP_USERNAME: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);
pub static SMTP_PASSWORD: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);
pub static SMTP_FROM: GucSetting<Option<&CStr>> = GucSetting::<Option<&CStr>>::new(None);

pub fn init() {
    GucRegistry::define_string_guc(
        "smtp_client.server",
        "The SMTP server to use for sending emails",
        "The SMTP server to use for sending emails.",
        &SMTP_SERVER,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_int_guc(
        "smtp_client.port",
        "The port to use for the SMTP server",
        "The port to use for the SMTP server.",
        &SMTP_PORT,
        1,
        65535,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_bool_guc(
        "smtp_client.tls",
        "Whether to use TLS for the SMTP connection",
        "Whether to use TLS for the SMTP connection.",
        &SMTP_TLS,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_string_guc(
        "smtp_client.username",
        "The username to use for the SMTP server",
        "The username to use for the SMTP server.",
        &SMTP_USERNAME,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_string_guc(
        "smtp_client.password",
        "The password to use for the SMTP server",
        "The password to use for the SMTP server.",
        &SMTP_PASSWORD,
        GucContext::Suset,
        GucFlags::SUPERUSER_ONLY,
    );

    GucRegistry::define_string_guc(
        "smtp_client.from_address",
        "The address the email should be sent from",
        "The address the email should be sent from.",
        &SMTP_FROM,
        GucContext::Suset,
        GucFlags::default(),
    );
}

pub fn get_smtp_server() -> Option<String> {
    handle_cstr(SMTP_SERVER.get())
}

pub fn get_smtp_port() -> u16 {
    SMTP_PORT.get() as u16
}

pub fn get_smtp_tls() -> bool {
    SMTP_TLS.get()
}

pub fn get_smtp_username() -> Option<String> {
    handle_cstr(SMTP_USERNAME.get())
}

pub fn get_smtp_password() -> Option<String> {
    handle_cstr(SMTP_PASSWORD.get())
}

pub fn get_smtp_from() -> Option<String> {
    handle_cstr(SMTP_FROM.get())
}

fn handle_cstr(val: Option<&CStr>) -> Option<String> {
    if let Some(cstr) = val {
        if let Ok(s) = cstr.to_str() {
            return Some(s.to_owned());
        }
    }
    None
}
