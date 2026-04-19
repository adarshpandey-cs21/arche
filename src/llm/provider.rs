use super::types::{GenerateRequest, GenerateResponse, StreamChunk};
use crate::error::AppError;
use futures::Stream;
use std::future::Future;
use std::pin::Pin;

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, AppError>> + Send>>;

type StreamGenerateFuture<'a> =
    Pin<Box<dyn Future<Output = Result<LlmStream, AppError>> + Send + 'a>>;

type GenerateFuture<'a> =
    Pin<Box<dyn Future<Output = Result<GenerateResponse, AppError>> + Send + 'a>>;

pub trait LlmProvider: Send + Sync {
    fn generate<'a>(&'a self, request: &'a GenerateRequest) -> GenerateFuture<'a>;

    fn stream_generate<'a>(&'a self, request: &'a GenerateRequest) -> StreamGenerateFuture<'a>;
}
