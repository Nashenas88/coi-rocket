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
impl<'a, 'r> FromRequest<'a, 'r> for &'a ScopedContainer {
    type Error = Error;

    fn from_request(req: &'a Request<'r>) -> Outcome<&'a ScopedContainer, Error> {
        req.local_cache(|| {
            let container = req.guard::<State<Container>>().succeeded()?;
            Some(ScopedContainer(container.scoped()))
        })
        .as_ref()
        .into_outcome((Status::InternalServerError, Error::MissingContainer))
    }
}

// For every injected param, just us the local cached scoped container
impl<'a, 'r, T, K> FromRequest<'a, 'r> for Injected<Arc<T>, K>
where
    T: Inject + ?Sized,
    K: ContainerKey<T>,
{
    type Error = Error;

    fn from_request(req: &'a Request<'r>) -> Outcome<Injected<Arc<T>, K>, Error> {
        let container = match req.guard::<&ScopedContainer>() {
            Outcome::Success(container) => container,
            Outcome::Failure(f) => return Outcome::Failure(f),
            Outcome::Forward(f) => return Outcome::Forward(f),
        };
        container.0
            .resolve::<T>(<K as ContainerKey<T>>::KEY)
            .map(Injected::new)
            .map_err(Error::Coi)
            .into_outcome(Status::InternalServerError)
    }
}
