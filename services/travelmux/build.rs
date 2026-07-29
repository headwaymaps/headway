//! Registers OTP's GTFS GraphQL schema with cynic, so that the query structs in
//! `src/otp/` can be checked against it at compile time.
//!
//! `schemas/otp.graphql` is the schema of the OTP version we deploy (see
//! `bin/_headway_version.sh`). See the README for how to refresh it when updating OTP.

fn main() {
    cynic_codegen::register_schema("otp")
        .from_sdl_file("schemas/otp.graphql")
        .expect("failed to read OTP GraphQL schema")
        .as_default()
        .expect("failed to register OTP GraphQL schema");
}
