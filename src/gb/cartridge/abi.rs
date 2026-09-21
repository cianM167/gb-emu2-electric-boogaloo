//! Host-side plugin ABI for a WASM-based Game Boy *cartridge* plugin.
//!
//! A plugin emulates the cartridge only: the mapper chip's ROM/RAM
//! read/write behavior, and (optionally) driving the analog Vin line
//! with raw audio, exactly as a real cart's audio-in circuitry would.
//!
//! Plugin module MUST export:
//!   fn plugin_abi_version() -> u32
//!   fn cart_init(rom_ptr: u32, rom_len: u32) -> i32   // 0 == success
//!   fn cart_read(addr: i32) -> i32                    // low 16/8 bits used
//!   fn cart_write(addr: i32, value: i32)               // low 16/8 bits used
//!   memory                                             // exported linear memory
//!
//! Plugin module MAY export (absent == cart has no Vin audio-in circuit):
//!   fn tick_vin_frame()                                // called once/frame by host
//!
//! Host provides as imports (module "env"):
//!   fn host_push_vin_samples(ptr: u32, len: u32)
//!   fn host_log(ptr: u32, len: u32)

use std::cell::RefCell;

use anyhow::{bail, ensure};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use wasmtime::{Caller, Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

use crate::gb::cartridge::mapper::Mapper;

pub const PLUGIN_ABI_VERSION: u32 = 1;

// where the rom is loaded into the plugins memory
const ROM_LOAD_OFFSET: u32 = 0x1000;

// convert errors implementing display into anyhow errors
fn wrap_wasm_err<T, E: std::fmt::Display>(result: Result<T, E>, msg: &str) -> anyhow::Result<T> {
    result.map_err(|e| anyhow::anyhow!("{msg}: {e}"))
}

// used to push samples to the internal apu for mixing
struct VinProducer(HeapProd<i16>);

// consumer side of audio samples
pub struct VinConsumer(HeapCons<i16>);

impl VinConsumer {
    /// Pulls one Vin sample for the current mixing tick, or `0` (silence)
    #[inline]
    pub fn pull_sample(&mut self) -> i16 {
        self.0.try_pop().unwrap_or(0)
    }
}

struct HostState {
    vin: VinProducer,
}

/// A loaded, version-checked cartridge plugin with its hot-path functions
/// resolved once at load time rather than looked up by name per call.
pub struct CartridgePlugin {
    store: RefCell<Store<HostState>>,
    #[allow(dead_code)] // kept alive for debugging
    instance: Instance,
    #[allow(dead_code)]
    memory: Memory,

    cart_read: TypedFunc<i32, i32>,
    cart_write: TypedFunc<(i32, i32), ()>,
    tick_vin_frame: Option<TypedFunc<(), ()>>,
}

impl CartridgePlugin {
    /// Loads a cartridge plugin from `path`, checks its declared ABI
    /// version, copies `rom` into its linear memory, and calls
    /// `cart_init`. Returns the plugin along with the `VinConsumer` the
    /// host's APU should pull from during mixing.
    ///
    /// `vin_capacity` is the ring buffer size in samples — size it to a
    /// small fraction of a frame (e.g. a few ms of audio at your Vin
    /// sample rate); it's a live passthrough line, not a bulk buffer.
    pub fn load(path: &str, rom: &[u8], vin_capacity: usize, ram_len: u32) -> anyhow::Result<(Self, VinConsumer)> {
        let engine = Engine::default();
        let module = wrap_wasm_err(
            Module::from_file(&engine, path),
            &format!("failed to load cartridge plugin at {path}"),
        )?;

        let vin_rb = HeapRb::<i16>::new(vin_capacity);
        let (vin_prod, vin_cons) = vin_rb.split();

        let mut store = Store::new(
            &engine,
            HostState {
                vin: VinProducer(vin_prod),
            },
        );
        let mut linker: Linker<HostState> = Linker::new(&engine);
        register_host_imports(&mut linker)?;

        let instance = wrap_wasm_err(
            linker.instantiate(&mut store, &module),
            "failed to instantiate cartridge plugin module",
        )?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow::anyhow!("plugin does not export linear memory named \"memory\""))?;

        // --- ABI version negotiation -------------------------------------
        let version_fn: TypedFunc<(), u32> = wrap_wasm_err(
            instance.get_typed_func(&mut store, "plugin_abi_version"),
            "plugin missing plugin_abi_version export",
        )?;
        let version = wrap_wasm_err(version_fn.call(&mut store, ()), "plugin_abi_version call trapped")?;
        ensure!(
            version == PLUGIN_ABI_VERSION,
            "plugin ABI version mismatch: host expects {PLUGIN_ABI_VERSION}, plugin reports {version}"
        );

        // --- ROM load + init ------------------------------------------------
        ensure!(
            (ROM_LOAD_OFFSET as usize) + rom.len() <= memory.data_size(&store),
            "plugin memory too small for ROM: need {} bytes at offset {}, have {}",
            rom.len(),
            ROM_LOAD_OFFSET,
            memory.data_size(&store)
        );
        wrap_wasm_err(
            memory.write(&mut store, ROM_LOAD_OFFSET as usize, rom),
            "failed writing ROM into plugin memory",
        )?;

        let init_fn: TypedFunc<(u32, u32, u32), i32> = wrap_wasm_err(
            instance.get_typed_func(&mut store, "cart_init"),
            "plugin missing cart_init export",
        )?;
        let result = wrap_wasm_err(
            init_fn.call(&mut store, (ROM_LOAD_OFFSET, rom.len() as u32, ram_len)),
            "cart_init call trapped",
        )?;
        if result != 0 {
            bail!("cart_init returned error code {result}");
        }

        // --- cache hot-path functions ---------------------------------------
        let cart_read: TypedFunc<i32, i32> = wrap_wasm_err(
            instance.get_typed_func(&mut store, "cart_read"),
            "plugin missing cart_read export",
        )?;
        let cart_write: TypedFunc<(i32, i32), ()> = wrap_wasm_err(
            instance.get_typed_func(&mut store, "cart_write"),
            "plugin missing cart_write export",
        )?;

        // Optional: most carts have no audio-in circuitry at all, so this
        // export is allowed to be absent. VinConsumer::pull_sample() just
        // returns silence in that case.
        let tick_vin_frame: Option<TypedFunc<(), ()>> =
            instance.get_typed_func(&mut store, "tick_vin_frame").ok();

        let plug_store = RefCell::new(store);

        let plugin = Self {
            store: plug_store,
            instance,
            memory,
            cart_read,
            cart_write,
            tick_vin_frame,
        };
        Ok((plugin, VinConsumer(vin_cons)))
    }

    /// Not part of `Mapper` — driven by the host's APU mixer once per
    /// frame, not by CPU bus reads/writes.
    #[inline]
    pub fn tick_vin_frame(&self) {
        if let Some(f) = &self.tick_vin_frame {
            let mut store = self.store.borrow_mut();
            f.call(&mut *store, ())
                .unwrap_or_else(|e| panic!("cartridge plugin tick_vin_frame trapped: {e}"));
        }
    }
}

impl Mapper for CartridgePlugin {
    fn read(&self, addr: u16) -> u8 {
        let mut store = self.store.borrow_mut();
        let result = self
            .cart_read
            .call(&mut *store, addr as i32)
            .unwrap_or_else(|e| panic!("cartridge plugin cart_read trapped: {e}"));
        result as u8
    }

    fn write(&mut self, addr: u16, value: u8) {
        let mut store = self.store.borrow_mut();
        self.cart_write
            .call(&mut *store, (addr as i32, value as i32))
            .unwrap_or_else(|e| panic!("cartridge plugin cart_write trapped: {e}"));
    }
}

/// Registers the `env` module imports the cartridge plugin can call into
fn register_host_imports(linker: &mut Linker<HostState>) -> anyhow::Result<()> {
    wrap_wasm_err(
        linker.func_wrap(
            "env",
            "host_push_vin_samples",
            |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| {
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => {
                        eprintln!("[cart plugin] host_push_vin_samples: no exported memory");
                        return;
                    }
                };

                let byte_len = match (len as usize).checked_mul(2) {
                    Some(n) => n,
                    None => {
                        eprintln!("[cart plugin] host_push_vin_samples: length overflow");
                        return;
                    }
                };
                let mut buf = vec![0u8; byte_len];
                if let Err(e) = memory.read(&caller, ptr as usize, &mut buf) {
                    eprintln!("[cart plugin] host_push_vin_samples: out-of-bounds read: {e}");
                    return;
                }

                let samples = buf.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]));

                caller.data_mut().vin.0.push_iter(samples);
            },
        ),
        "failed to register host_push_vin_samples import",
    )?;

    wrap_wasm_err(
        linker.func_wrap(
            "env",
            "host_log",
            |mut caller: Caller<'_, HostState>, ptr: u32, len: u32| {
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => {
                        eprintln!("[cart plugin] host_log: no exported memory");
                        return;
                    }
                };

                let mut buf = vec![0u8; len as usize];
                if let Err(e) = memory.read(&caller, ptr as usize, &mut buf) {
                    eprintln!("[cart plugin] host_log: out-of-bounds read: {e}");
                    return;
                }

                eprintln!("[cart plugin] {}", String::from_utf8_lossy(&buf));
            },
        ),
        "failed to register host_log import",
    )?;

    Ok(())
}