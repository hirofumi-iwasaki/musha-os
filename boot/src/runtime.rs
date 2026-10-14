// SPDX-License-Identifier: Apache-2.0
//! OS-owned, scoped cooperative service session. USB Hosts remain on OS stack
//! frames until all callbacks finish and DMA is quiesced. Not a Ring 3 runtime.
use crate::app::{Diagnostic, VIEWER};
use musha_api::{Application, Context, Error, Step};
fn launch(
    app: &mut Diagnostic,
    ctx: &mut Context<'_>,
    time: &mut crate::acpi::Time,
    initialized: &mut bool,
) -> Result<(), Error> {
    if *initialized {
        return Ok(());
    }
    ctx.advance(time.now().map_err(|_| Error::Io)?)?;
    let required = ctx.version() == musha_api::API_VERSION
        && ctx.arena_bytes() > 0
        && ctx.screen_size().0 >= 320;
    let mut accepted = false;
    crate::bootstate::update(|r, t| accepted = r.ready(required, t));
    if !accepted {
        return Err(Error::Unsupported);
    }
    crate::debug(
        if crate::bootstate::report().files == musha_platform::readiness::Service::Live {
            b"MUSHA: OS_READY STATIC_COOPERATIVE FILE_VIEWER\n"
        } else {
            b"MUSHA: OS_READY STATIC_COOPERATIVE RECOVERY_DISPLAY\n"
        },
    );
    // Exercise the actual required API before handing execution to the app.
    let _ = (
        ctx.version(),
        ctx.now_ms(),
        ctx.screen_size(),
        ctx.arena_bytes(),
        ctx.next_key(),
    );
    crate::bootstate::update(|r, t| {
        r.entering(t);
    });
    crate::debug(b"MUSHA: APP_ENTERING\n");
    app.init(ctx)?;
    *initialized = true;
    Ok(())
}
pub(crate) fn run(arena: &mut [u8], info: &crate::BootInfo) -> Result<(), Error> {
    // Framebuffer, cached file and arena have a single application owner.
    let mut ctx = unsafe { Context::new(arena, info.framebuffer) }?;
    let mut app = Diagnostic::new();
    let mut initialized = false;
    let mut network = None;
    let mut steps = 0u64;
    let mut max_gap = 0u64;
    let mut last_cycle = None;
    let mut over_budget = 0u64;
    let mut keyboard_missing = false;
    let result = (|| {
        let mut time = crate::acpi::Time::new(info.timer).map_err(|_| Error::Unsupported)?;
        let mut ready = false;
        let mut had_keyboard = false;
        let mut input_error = None;
        let mut last_udp = 0;
        // External slot 0 has priority; slot 1 is reserved for a future T2 adapter.
        let mut inputs = musha_input::sources::Sources::<2>::default();
        let mut external = None;
        let mut cycle = |event: crate::xhci::AppEvent<'_>| -> Result<bool, &'static str> {
            let events = match event {
                crate::xhci::AppEvent::Files(files) => {
                    #[cfg(external_app)]
                    {
                        let now = time.now().map_err(|_| "APP INPUT ERROR")?;
                        match crate::external::step(files, now, info.framebuffer) {
                            musha_api::bootfile::State::Absent => {
                                if cfg!(feature = "diagnostics-detail") {
                                    crate::diagnostics::observation(format_args!(
                                        "DIAGNOSTIC HOLD ELF ABSENT"
                                    ));
                                    crate::debug(b"MUSHA: DIAGNOSTIC_HOLD ELF_ABSENT\n");
                                    return Ok(true);
                                }
                                app.viewer.bootfile_absent();
                                launch(&mut app, &mut ctx, &mut time, &mut initialized)
                                    .map_err(|_| "APP INPUT ERROR")?;
                            }
                            musha_api::bootfile::State::Ready(_)
                            | musha_api::bootfile::State::Rejected(_) => return Ok(true),
                            _ => return Ok(false),
                        }
                    }
                    if !initialized {
                        return Err("APP INPUT ERROR");
                    }
                    if let Err(error) = app.files(&mut ctx, files) {
                        input_error = Some(error);
                        return Err("APP INPUT ERROR");
                    }
                    return Ok(app.exit_requested);
                }
                crate::xhci::AppEvent::Retained(_) => return Err("UNHANDLED STORAGE LOOP"),
                crate::xhci::AppEvent::File(bytes) => {
                    ctx.install_boot_file(bytes).map_err(|_| "APP FILE CACHE")?;
                    return Ok(false);
                }
                crate::xhci::AppEvent::FileError(error) => {
                    ctx.record_file_error(error);
                    return Ok(false);
                }
                crate::xhci::AppEvent::Ready(input) => {
                    #[cfg(external_app)]
                    if crate::external::claimed() {
                        return Ok(true);
                    }
                    if cfg!(all(external_app, feature = "diagnostics-detail")) {
                        let no_medium = crate::bootstate::report().files
                            != musha_platform::readiness::Service::Live;
                        if no_medium {
                            crate::diagnostics::observation(format_args!(
                                "DIAGNOSTIC HOLD NO MEDIUM"
                            ));
                            crate::debug(b"MUSHA: DIAGNOSTIC_HOLD NO_MEDIUM\n");
                        }
                        return Ok(no_medium);
                    }
                    if initialized {
                        return Ok(false);
                    }
                    if VIEWER {
                        crate::diagnostics::hide(true);
                    }
                    ctx.finish_file_discovery(false);
                    if input {
                        external = Some(
                            inputs
                                .connect(0, |k, d| ctx.push_key(k, d))
                                .map_err(|_| "INPUT SOURCE CONNECT")?,
                        );
                    }
                    ctx.set_input_active(inputs.active().is_some());
                    had_keyboard = input;
                    match crate::net::Session::start(info, &mut time) {
                        Ok(session) => {
                            network = session;
                            crate::bootstate::update(|r, _| {
                                r.network = if network.is_some() {
                                    musha_platform::readiness::Service::Live
                                } else {
                                    musha_platform::readiness::Service::Unavailable
                                };
                            });
                        }
                        Err(error) => crate::net::failed(error),
                    }
                    if !cfg!(external_app)
                        || crate::bootstate::report().files
                            != musha_platform::readiness::Service::Live
                    {
                        if let Err(error) = launch(&mut app, &mut ctx, &mut time, &mut initialized)
                        {
                            input_error = Some(error);
                            return Err("APP INPUT ERROR");
                        }
                    }
                    crate::bootstate::update(|r, _| {
                        r.usb = musha_platform::readiness::Service::Live;
                        r.network = if network.is_some() {
                            musha_platform::readiness::Service::Live
                        } else {
                            musha_platform::readiness::Service::Unavailable
                        };
                    });
                    ready = true;
                    crate::debug(b"MUSHA: RUNTIME_POLL_READY\n");
                    &[][..]
                }
                crate::xhci::AppEvent::Keys(events) => events,
            };
            // The viewer cannot end a no-keyboard session while boot-file I/O
            // is pending. No file service still follows the existing viewer path.
            #[cfg(external_app)]
            if crate::external::waiting() {
                return Ok(false);
            }
            let step = (|| {
                let before = time.now().map_err(|_| Error::Io)?;
                if let Some(last) = last_cycle {
                    max_gap = max_gap.max(before - last);
                }
                last_cycle = Some(before);
                ctx.advance(before)?;
                if let Some(session) = network.as_mut() {
                    if let Err(error) = session.poll(before) {
                        crate::net::failed(error);
                        network = None;
                        crate::bootstate::update(|r, _| {
                            r.network = musha_platform::readiness::Service::Stopped
                        });
                        app.recheck_after_error = true;
                    }
                }
                if let Some(session) = network.as_ref() {
                    let udp = session.udp_count();
                    if udp != last_udp {
                        last_udp = udp;
                        app.request_file_recheck();
                    }
                }
                for &(key, down) in events {
                    let source = external.ok_or(Error::Invalid)?;
                    inputs
                        .key(source, key, down, |k, d| ctx.push_key(k, d))
                        .map_err(|_| Error::Invalid)?;
                }
                let state = app.step(&mut ctx)?;
                steps += 1;
                if time.now().map_err(|_| Error::Io)? - before > 1 {
                    over_budget += 1;
                    if over_budget == 1 {
                        crate::debug(b"MUSHA: APP_STEP_BUDGET_EXCEEDED\n");
                    }
                }
                Ok(app.exit_requested || state == Step::Complete)
            })();
            match step {
                Ok(stop) => Ok(stop),
                Err(error) => {
                    input_error = Some(error);
                    Err("APP INPUT ERROR")
                }
            }
        };
        let usb_result = crate::xhci::run_session(info, &mut cycle);
        drop(cycle);
        crate::bootstate::update(|r, _| {
            r.usb = if usb_result == Err("QUIESCE FAILED") {
                musha_platform::readiness::Service::StopUnconfirmed
            } else {
                musha_platform::readiness::Service::Stopped
            };
            if r.files == musha_platform::readiness::Service::Live {
                r.files = r.usb;
            }
        });
        #[cfg(external_app)]
        {
            crate::external::session_ended(usb_result.is_ok());
            if crate::external::claimed() {
                // Never consume/reuse the arena after an unconfirmed DMA stop.
                if usb_result == Err("QUIESCE FAILED") {
                    return Err(Error::Io);
                }
                return Ok(());
            }
        }
        if cfg!(all(external_app, feature = "diagnostics-detail")) {
            crate::diagnostics::observation(format_args!(
                "DIAGNOSTIC HOLD SESSION {:?}",
                usb_result
            ));
            return if usb_result == Err("QUIESCE FAILED") {
                Err(Error::Io)
            } else {
                Ok(())
            };
        }
        // Only a successful discovery with no input is a missing keyboard.
        // Hardware/application errors must retain the generic session status.
        keyboard_missing = ready && !had_keyboard && usb_result.is_ok() && input_error.is_none();
        if input_error.is_none() && usb_result != Err("QUIESCE FAILED") {
            app.files_closed(&mut ctx);
        }
        crate::debug(b"MUSHA: FILE_SERVICE_CLOSED\n");
        // Every diagnose return (Esc, deadline, disconnect, hardware/app error)
        // detaches the input source. This changes CPU input state only; xHCI owns
        // DMA quiescence. Do not invoke a failed app again solely for cleanup.
        if let Some(source) = external.take() {
            inputs
                .disconnect(source, |k, d| {
                    ctx.push_key(k, d);
                    crate::debug(b"MUSHA: INPUT_DETACH_KEY_UP=");
                    crate::debug(&crate::cpu::hex(k as u64));
                    crate::debug(b"\n");
                })
                .map_err(|_| Error::Invalid)?;
        }
        crate::debug(b"MUSHA: INPUT_SOURCE_DETACHED\n");
        let input_completed = had_keyboard && usb_result.is_ok();
        ctx.finish_file_discovery(usb_result.is_err());
        ctx.set_input_active(inputs.active().is_some());
        if let Some(error) = input_error {
            return Err(error);
        }
        if !initialized {
            if VIEWER {
                crate::diagnostics::hide(true);
            }
            match crate::net::Session::start(info, &mut time) {
                Ok(session) => {
                    network = session;
                    crate::bootstate::update(|r, _| {
                        r.network = if network.is_some() {
                            musha_platform::readiness::Service::Live
                        } else {
                            musha_platform::readiness::Service::Unavailable
                        };
                    });
                }
                Err(error) => crate::net::failed(error),
            }
            launch(&mut app, &mut ctx, &mut time, &mut initialized)?;
            crate::debug(b"MUSHA: RUNTIME_POLL_READY NO_INPUT\n");
        }
        // With no keyboard, progress continues for a bounded diagnostic window.
        // Esc or completion of the debug keyboard window stops all services.
        let tail_started = time.now().map_err(|_| Error::Io)?;
        for _ in 0..100_000_000 {
            let before = time.now().map_err(|_| Error::Io)?;
            if let Some(last) = last_cycle {
                max_gap = max_gap.max(before - last);
            }
            last_cycle = Some(before);
            ctx.advance(before)?;
            if let Some(session) = network.as_mut() {
                if let Err(error) = session.poll(before) {
                    crate::net::failed(error);
                    network = None;
                    crate::bootstate::update(|r, _| {
                        r.network = musha_platform::readiness::Service::Stopped
                    });
                    app.recheck_after_error = true;
                }
            }
            if let Some(session) = network.as_ref() {
                let udp = session.udp_count();
                if udp != last_udp {
                    last_udp = udp;
                    app.request_file_recheck();
                }
            }
            let state = app.step(&mut ctx)?;
            steps += 1;
            let expired = network
                .as_ref()
                .is_none_or(|session| before - session.started() >= 10000);
            if state == Step::Complete && (input_completed || app.exit_requested || expired) {
                return Ok(());
            }
            if before - tail_started >= 11000 {
                return Err(Error::Io);
            }
        }
        Err(Error::Io)
    })();
    if result.is_err() {
        app.complete = false;
    }
    if initialized {
        app.shutdown(&mut ctx);
    }
    if result.is_ok() && !VIEWER {
        if let Some(session) = network.as_mut() {
            session.finish(info.framebuffer);
        }
    }
    drop(network); // Also runs on application/clock errors, before propagating.
    crate::bootstate::update(|r, _| {
        if r.network == musha_platform::readiness::Service::Live {
            r.network = musha_platform::readiness::Service::Stopped;
        }
        #[cfg(external_app)]
        if crate::external::claimed() {
            return;
        }
        r.finish(result.is_ok());
    });
    crate::bootstate::dump();
    #[cfg(external_app)]
    if crate::external::claimed() {
        return result;
    }
    if cfg!(all(external_app, feature = "diagnostics-detail")) {
        return result;
    }
    if VIEWER {
        // Final backend status drawing can overlap the UI. Repaint after all
        // device owners have stopped; preserve the unavailable/ended screen.
        app.viewer.closed(result.is_ok() && keyboard_missing);
        while !app.viewer.painted() {
            app.viewer.render_step(&mut ctx)?;
        }
    }
    result?;
    crate::debug(b"MUSHA: RUNTIME_COOPERATIVE_OK STEPS=");
    crate::debug(&crate::cpu::hex(steps));
    crate::debug(b" MAX_GAP_MS=");
    crate::debug(&crate::cpu::hex(max_gap));
    crate::debug(b" FILE_RECHECKS=");
    crate::debug(&crate::cpu::hex(app.file_checks.saturating_sub(1)));
    crate::debug(b" OVER_BUDGET=");
    crate::debug(&crate::cpu::hex(over_budget));
    crate::debug(b"\n");
    crate::debug(b"MUSHA: APP_LIFECYCLE_OK STEPS=");
    crate::debug(&crate::cpu::hex(steps));
    crate::debug(b"\n");
    Ok(())
}
