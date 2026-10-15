use serde::Serialize;

pub mod account;
pub mod otp;

#[derive(Debug, thiserror::Error, Serialize, ts_rs::TS)]
#[serde(tag = "code")]
pub enum AppError {
    #[error("Cofre bloqueado")]
    Locked,
    #[error("Operação cancelada")]
    Cancelled,
    #[error("Campo inválido: {field}")]
    InvalidInput { field: &'static str },
    #[error("Formato não suportado")]
    UnsupportedFormat,
    #[error("Armazenamento seguro indisponível")]
    StorageUnavailable,
    #[error(
        "Esta versão não tem acesso autorizado ao Keychain. Atualize a identidade do cofre pela migração; nenhuma senha será solicitada automaticamente"
    )]
    KeychainAccessDenied,
    #[error("Conta não encontrada")]
    NotFound,
    #[error("Contador esgotado")]
    CounterExhausted,
    #[error("Senha do backup incorreta")]
    InvalidPassword,
    #[error("Limite de processamento excedido")]
    LimitExceeded,
    #[error("Dados foram alterados; revise novamente")]
    Conflict,
    #[error(
        "Permita a gravação da tela nos Ajustes do Sistema para escanear um QR; pode ser necessário reabrir o app"
    )]
    ScreenCaptureDenied,
    #[error("Não foi possível capturar a região escolhida")]
    CaptureUnavailable,
    #[error("Nenhum QR Code encontrado na região escolhida")]
    NoQrCode,
    #[error("O QR Code não contém uma conta OTP válida")]
    InvalidQrCode,
    #[error("A captura excedeu o tempo disponível; tente novamente")]
    CaptureTimedOut,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn invalid(field: &'static str) -> Self {
        Self::InvalidInput { field }
    }
}
