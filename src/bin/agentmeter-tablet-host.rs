//! Operator-run transport probe. Provider values are explicitly mock data.
use agentmeter_p0::tablet::{ServerConfig, TabletServer};
use agentmeter_p0::usb::{OwnedReverse, ReverseMappingState, launch_device_browser, setup_reverse};
use std::{
    io::{self, BufRead, IsTerminal, Write},
    path::PathBuf,
    time::Duration,
};

struct UsbConfig {
    adb: PathBuf,
    serial: String,
    device_port: u16,
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut mock = false;
    let mut config = ServerConfig::default();
    let mut adb = None;
    let mut serial = None;
    let mut device_port = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mock-providers" => mock = true,
            "--adb" => {
                adb = Some(PathBuf::from(
                    args.next().ok_or("--adb requires an absolute path")?,
                ));
            }
            "--serial" => {
                serial = Some(args.next().ok_or("--serial requires a device serial")?);
            }
            "--device-port" => {
                device_port = Some(
                    args.next()
                        .ok_or("--device-port requires a number from 1 to 65535")?
                        .parse::<u16>()
                        .ok()
                        .filter(|port| *port != 0)
                        .ok_or("--device-port requires a number from 1 to 65535")?,
                );
            }
            "--port" => {
                config.listen_port = args
                    .next()
                    .ok_or("--port requires a number from 1 to 65535")?
                    .parse::<u16>()
                    .ok()
                    .filter(|port| *port != 0)
                    .ok_or("--port requires a number from 1 to 65535")?;
            }
            "--pair-store" => {
                let path = std::path::PathBuf::from(
                    args.next()
                        .ok_or("--pair-store requires an absolute path")?,
                );
                if !path.is_absolute() {
                    return Err("--pair-store requires an absolute path".into());
                }
                config.pair_store_path = Some(path);
            }
            "--help" => {
                println!(
                    "agentmeter-tablet-host --mock-providers [--port 1..65535] [--pair-store ABSOLUTE_PATH] [--adb ABSOLUTE_PATH --serial SERIAL --device-port 1..65535]\nCommands on stdin: pair, open, status, recover-usb, clear-pairing, quit. EOF shuts down.\nMock Provider data only; an accepted browser launch is not an authenticated health signal.\nPair codes are shown only in an interactive terminal, never redirected output."
                );
                return Ok(());
            }
            _ => return Err("unknown argument; use --help".into()),
        }
    }
    if !mock {
        return Err(
            "this probe requires explicit --mock-providers; live Providers are not connected"
                .into(),
        );
    }
    let usb = match (adb, serial, device_port) {
        (None, None, None) => None,
        (Some(adb), Some(serial), Some(device_port)) => Some(UsbConfig {
            adb,
            serial,
            device_port,
        }),
        _ => {
            return Err(
                "USB mode requires --adb, --serial and --device-port together; use --help".into(),
            );
        }
    };
    let durable = config.pair_store_path.is_some();
    let server = TabletServer::start(config).map_err(|error| {
        if error.kind() == io::ErrorKind::AddrInUse {
            "requested loopback port is occupied; stop its owner or explicitly choose another port"
        } else {
            "tablet listener or protected pair store could not start; check port and store permissions"
        }
    })?;
    let mut mapping: Option<OwnedReverse> = if let Some(usb) = &usb {
        let (_, mapping) = setup_reverse(
            &usb.adb,
            &usb.serial,
            usb.device_port,
            server.addr().port(),
            Duration::from_secs(5),
        )
        .map_err(|failure| format!("USB setup failed ({}): {}", failure.code, failure.message))?;
        Some(mapping)
    } else {
        None
    };
    let usb_status = mapping.as_ref().map(|mapping| {
        serde_json::json!({
            "selected_serial": mapping.serial,
            "transport": "usb",
            "device_port": mapping.device_port,
            "host_port": mapping.host_port,
            "mapping": "verified",
            "browser_launch": "not_requested",
            "authenticated_health": "not_observed"
        })
    });
    println!(
        "{}",
        serde_json::json!({"event":"ready","origin":server.origin(),"provider_data":"mock","durable_pairing":durable,"usb":usb_status})
    );
    io::stdout()
        .flush()
        .map_err(|_| "cannot announce host readiness")?;
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    let mut browser_launch_requested = false;
    for line in io::stdin().lock().lines() {
        match line.map_err(|_| "cannot read operator command")?.trim() {
            "quit" => break,
            "pair" if interactive => {
                // Local operator display only: no code in URL, readiness JSON or logs.
                println!("Pair code (valid 2 minutes): {}", server.rotate_pair_code());
            }
            "pair" => eprintln!("pair code suppressed: use an interactive local terminal"),
            "open" => {
                if let Some(usb) = &usb {
                    launch_device_browser(
                        &usb.adb,
                        &usb.serial,
                        usb.device_port,
                        Duration::from_secs(5),
                    )
                    .map_err(|failure| {
                        format!(
                            "browser launch failed ({}): {}",
                            failure.code, failure.message
                        )
                    })?;
                    browser_launch_requested = true;
                    eprintln!(
                        "browser launch requested; authenticated tablet health is still unobserved"
                    );
                } else {
                    eprintln!("open requires USB mode with --adb, --serial and --device-port");
                }
            }
            "status" => {
                let mapping_status = mapping.as_ref().map(|mapping| match mapping.inspect() {
                    Ok(state) => serde_json::json!({"state": state}),
                    Err(failure) => serde_json::json!({
                        "state": "unavailable",
                        "failure_code": failure.code,
                        "message": failure.message
                    }),
                });
                println!(
                    "{}",
                    serde_json::json!({
                        "event": "transport_status",
                        "usb_configured": usb.is_some(),
                        "usb_mapping": mapping_status,
                        "browser_launch_requested": browser_launch_requested,
                        "authenticated_activity": server.transport_status()
                    })
                );
                io::stdout()
                    .flush()
                    .map_err(|_| "cannot announce transport status")?;
            }
            "recover-usb" => {
                let Some(usb) = &usb else {
                    eprintln!("USB recovery is unavailable because USB mode is not configured");
                    continue;
                };
                let Some(state) = mapping.as_ref().map(OwnedReverse::inspect) else {
                    eprintln!("USB recovery is unavailable because no mapping is owned");
                    continue;
                };
                match state {
                    Ok(ReverseMappingState::Owned) => {
                        eprintln!("USB mapping is already verified; recovery was not needed");
                    }
                    Ok(ReverseMappingState::Changed) => {
                        eprintln!(
                            "USB mapping changed externally; refusing to replace or remove it"
                        );
                    }
                    Ok(ReverseMappingState::Missing) => {
                        mapping.take().expect("checked mapping").abandon();
                        match setup_reverse(
                            &usb.adb,
                            &usb.serial,
                            usb.device_port,
                            server.addr().port(),
                            Duration::from_secs(5),
                        ) {
                            Ok((_, recovered)) => {
                                mapping = Some(recovered);
                                eprintln!(
                                    "USB mapping recovered; browser launch and authenticated activity must be checked separately"
                                );
                            }
                            Err(failure) => eprintln!(
                                "USB recovery failed ({}): {}",
                                failure.code, failure.message
                            ),
                        }
                    }
                    Err(failure) => eprintln!(
                        "USB mapping cannot be inspected ({}): {}",
                        failure.code, failure.message
                    ),
                }
            }
            "clear-pairing" => {
                server
                    .clear_pairing()
                    .map_err(|_| "pair revocation persistence failed; authorization is disabled")?;
                eprintln!("pairing cleared");
            }
            "" => {}
            _ => {
                eprintln!(
                    "unknown operator command; use pair, open, status, recover-usb, clear-pairing or quit"
                )
            }
        }
    }
    drop(mapping);
    drop(server);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
