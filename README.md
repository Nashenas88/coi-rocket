# coi-rocket

[![Build Status](https://travis-ci.org/Nashenas88/coi-rocket.svg?branch=master)](https://travis-ci.org/Nashenas88/coi-rocket)
[![docs.rs](https://docs.rs/coi-rocket/badge.svg)](https://docs.rs/coi-rocket)
[![crates.io](https://img.shields.io/crates/v/coi-rocket.svg)](https://crates.io/crates/coi-rocket)

Dependency Injection in Rust

This crate provides integration support between `coi` and `rocket`. It
exposes an `inject` procedural attribute macro to generate the code for
retrieving your dependencies from a `Container` registered with `rocket`.

## Example

```rust,no_run
// What's needed for the example fn below
use rocket::get;
use rocket_contrib::json::Json;
use std::sync::Arc;

// Add the `inject` attribute to the function you want to inject
// What this crate provides
#[coi_rocket::inject]
#[get("/<id>")]
fn get(
    id: u64,
    // Add the `inject` field attribute to each attribute you want
    // injected
    #[inject] service: Arc<dyn IService>
) -> Result<Json<DataDto>, ()> {
    let data = service.get(id)?;
    Ok(Json(DataDto::from(data)))
}

// Just data models for the above fn
use serde::Serialize;

#[derive(Serialize)]
struct DataDto {
    name: String,
}

impl DataDto {
    fn from(data: Data) -> Self {
        Self {
            name: data.name
        }
    }
}


// An example of what's usually needed to make effective use of this
// crate is below
use coi::Inject;

// This section shows coi being put to use
// It's very important that the version of coi and the version
// of coi-rocket used match since coi-rocket implements
// some coi traits

// Here we're marking a trait as injectable
trait IService: Inject {
    fn get(&self, id: u64) -> Result<Data, ()>;
}

// And here we're marking a type that's capable of providing the
// above trait
#[derive(Inject)]
#[coi(provides dyn IService with ServiceImpl::new(repo))]
struct ServiceImpl {
    // Here we're injecting a dependency. `ServiceImpl` does
    // not need to know how to get this value.
    #[coi(inject)]
    repo: Arc<dyn IRepo>
}

// Normal impl for struct
impl ServiceImpl {
    fn new(repo: Arc<dyn IRepo>) -> Self {
        Self { repo }
    }
}

// Normal impl of trait for struct
impl IService for ServiceImpl {
    fn get(&self, id: u64) -> Result<Data, ()> {
        self.repo.read_from_db(id)
    }
}

// The data that will be passed between services
struct Data {
    id: u64,
    name: String,
}

// Here's the trait from above
trait IRepo: Inject {
    fn read_from_db(&self, id: u64) -> Result<Data, ()>;
}

// And it's setup below
#[derive(Inject)]
#[coi(provides dyn IRepo with RepoImpl)]
struct RepoImpl;

impl IRepo for RepoImpl {
    fn read_from_db(&self, id: u64) -> Result<Data, ()> {
        Ok(Data {
            id,
            name: format!("{}'s name...", id)
        })
    }
}

#[launch]
fn rocket() -> _ {
    use rocket::{routes, Rocket};
    use coi::container;

    // Construct your coi container with your keys and providers
    // See the coi crate for more details
    let container = container!{
        repo => RepoImplProvider; scoped,
        service => ServiceImplProvider; scoped
    };

    Rocket::build()
        // Don't forget to manage the container so it can be used!
        .manage(container)
        .mount("/", routes![get])
}
```

See the repo [`coi-rocket-sample`] for a more involved example.

[`coi-rocket-sample`]: https://github.com/Nashenas88/coi-rocket-sample

#### License

<sup>
Licensed under either of <a href="LICENSE.Apache-2.0">Apache License, Version
2.0</a> or <a href="LICENSE.MIT">MIT license</a> at your option.
</sup>

<br/>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

`SPDX-License-Identifier: MIT OR Apache-2.0`
