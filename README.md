# memflow-winio

A memflow connector that exploits the vulnerable EneTechIo (WinIo) driver in Thermaltake's TOUGHRAM software (version
1.0.3), which can be found [here](https://github.com/a2x/memflow-winio/releases/download/0.1.0/winio64.sys).

## Building the Stand-alone Connector for Dynamic Loading

To compile a dynamic library for use with the connector inventory, use the following command:

```bash
cargo build --release
```

Alternatively, you can install it using memflowup:

1. Install memflowup:
    ```bash
   cargo install --git https://github.com/memflow/memflowup
    ```
2. Build the connector:
    ```bash
   memflowup build https://github.com/a2x/memflow-winio
    ```

## Statically Linking the Connector in a Rust Project

To statically link the connector in your Rust project, first add the following dependency to your Cargo.toml:

```toml
memflow-winio = { git = "https://github.com/a2x/memflow-winio" }
```

After adding the dependency, you can create a new connector instance like this:

```rust
use memflow::prelude::v1::*;
use memflow_win32::prelude::v1::*;

fn main() -> Result<()> {
    let connector = memflow_winio::create_connector(&ConnectorArgs::default())?;

    let mut kernel = Win32Kernel::builder(connector)
        .build_default_caches()
        .build()?;

    let process = kernel.process_by_name("explorer.exe")?;

    println!("{:#?}", process.info());

    Ok(())
}
```

## Service Management

By default, this crate enables the `auto-start` feature that manages the WinIo driver service lifecycle automatically.

When the connector is being initialized, it first checks that a service with the name specified in the
`WINIO_SERVICE_NAME` environment variable exists, and if it does, it will start it if it's not already running (if this
environment variable is not set, the service name defaults to `winio`). If a service with that name doesn't exist, the
connector will automatically create a new service with that name using the driver path from the `WINIO_DRIVER_PATH`
environment variable (if this environment variable is not set, the driver path defaults to `C:\winio64.sys`). For the
service to be created successfully, the driver file must exist.

This behavior can be disabled for cases where you prefer to create the service manually and ensure it's started before
initializing the connector by excluding the default features in your Cargo.toml, like so:

```toml
memflow-winio = { git = "https://github.com/a2x/memflow-winio", default-features = false }
```

## License

This project is licensed under the MIT license. See the [LICENSE](LICENSE) file for details.
