use async_trait::async_trait;
use coi::{Container, Inject};
use rocket::{
    http::Status,
    outcome::IntoOutcome as _,
    request::{FromRequest, Outcome},
    Request, State,
};
use std::{marker::PhantomData, sync::Arc};

pub use coi_rocket_derive::inject;

#[doc(hidden)]
pub trait ContainerKey<T>
where
    T: Inject + ?Sized,
{
    const KEY: &'static str;
}

#[doc(hidden)]
pub struct Injected<T, K>(pub T, pub PhantomData<K>);

impl<T, K> Injected<T, K> {
    #[doc(hidden)]
    pub fn new(injected: T) -> Self {
        Self(injected, PhantomData)
    }
}

struct ScopedContainer(Container);

#[derive(Debug)]
pub enum Error {
    Coi(coi::Error),
    MissingContainer,
}

// For every request that needs a container, create a scoped container that lives
// for the duration of that request.
#[async_trait]
impl<'r> FromRequest<'r> for &'r ScopedContainer {
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<&'r ScopedContainer, Error> {
        req.local_cache_async::<Option<ScopedContainer>, _>(async move {
            let container = req.guard::<&State<Container>>().await.succeeded()?;
            Some(ScopedContainer(container.scoped()))
        })
        .await
        .as_ref()
        .or_error((Status::InternalServerError, Error::MissingContainer))
    }
}

// For every injected param, just us the local cached scoped container
#[async_trait]
impl<'r, T, K> FromRequest<'r> for Injected<Arc<T>, K>
where
    T: Inject + ?Sized,
    K: ContainerKey<T>,
{
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Injected<Arc<T>, K>, Error> {
        let container = match req.guard::<&ScopedContainer>().await {
            Outcome::Success(container) => container,
            Outcome::Error(f) => return Outcome::Error(f),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };
        container
            .0
            .resolve::<T>(<K as ContainerKey<T>>::KEY)
            .map(Injected::new)
            .map_err(Error::Coi)
            .or_error(Status::InternalServerError)
    }
}
