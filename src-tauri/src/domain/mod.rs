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
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn invalid(field: &'static str) -> Self {
        Self::InvalidInput { field }
    }
}
