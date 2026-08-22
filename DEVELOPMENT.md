# Development

Any form of support is greatly appreciated. 

Please note the following things when creating pull requests:

1. Unit tests: Check that all unit tests are successful and that changes are covered by existing or new tests.
2. Code style: Apply the code style by executing `cargo fmt`

## Tests

The complete test suite includes executable documentation examples that act as integration tests.
A Redis server must be available at `127.0.0.1:6379`:

````
cargo test
````

The isolated unit tests do not require Redis:

````
cargo test --lib
````
