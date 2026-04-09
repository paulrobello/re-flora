mod app;
mod audio;
mod builder;
mod egui_renderer;
mod flora;
mod gameplay;
mod generated;
mod geom;
#[macro_use]
mod gui_adjustables;
mod particles;
mod procedual_placer;
mod resource;
mod tracer;
mod tree_gen;
mod util;
mod vkn;
mod wind;
mod window;

use app::AppController;
use env_logger::Env;
use winit::event_loop::EventLoop;

/// Application launch options parsed from CLI arguments.
#[derive(Clone, Debug)]
pub struct AppOptions {
    /// Run in windowed mode instead of borderless fullscreen.
    pub windowed: bool,
    /// Disable shadow rendering pass.
    pub no_shadows: bool,
    /// Disable denoiser passes.
    pub no_denoise: bool,
    /// Disable god ray pass.
    pub no_god_rays: bool,
    /// Disable lens flare passes.
    pub no_lens_flare: bool,
    /// Disable main tracer (black screen, for isolating other passes).
    pub no_tracer: bool,
    /// Disable particle simulation (butterflies, leaves).
    pub no_particles: bool,
    /// Disable flora/leaves graphics passes (grass, tree leaves).
    pub no_flora: bool,
    /// Path to save a screenshot after rendering starts. None = no screenshot.
    pub screenshot_path: Option<String>,
    /// Delay in seconds after rendering starts before taking the screenshot.
    pub screenshot_delay: f32,
    /// Auto-exit N seconds after rendering starts. None = don't auto-exit.
    pub auto_exit_delay: Option<f32>,
    /// Enable per-frame performance timing output to console.
    pub perf: bool,
}

impl AppOptions {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();

        let parse_f32_after = |flag: &str| -> Option<f32> {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1))
                .and_then(|v| v.parse::<f32>().ok())
        };

        let parse_string_after = |flag: &str| -> Option<String> {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };

        Self {
            windowed: args.iter().any(|a| a == "--windowed"),
            no_shadows: args.iter().any(|a| a == "--no-shadows"),
            no_denoise: args.iter().any(|a| a == "--no-denoise"),
            no_god_rays: args.iter().any(|a| a == "--no-god-rays"),
            no_lens_flare: args.iter().any(|a| a == "--no-lens-flare"),
            no_tracer: args.iter().any(|a| a == "--no-tracer"),
            no_particles: args.iter().any(|a| a == "--no-particles"),
            no_flora: args.iter().any(|a| a == "--no-flora"),
            screenshot_path: parse_string_after("--screenshot"),
            screenshot_delay: parse_f32_after("--screenshot-delay").unwrap_or(5.0),
            auto_exit_delay: parse_f32_after("--auto-exit"),
            perf: args.iter().any(|a| a == "--perf"),
        }
    }
}

#[allow(dead_code)]
fn backtrace_on() {
    use std::env;
    env::set_var("RUST_BACKTRACE", "1");
}

fn init_env_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or(
        "info,winit=warn,sctk=warn,wayland_client=warn,x11rb=warn,calloop=error,symphonia_format_riff=warn",
    ))
    .format(|buf, record| {
        use std::io::Write;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let local_time = chrono::DateTime::from_timestamp_millis(now as i64)
            .unwrap()
            .with_timezone(&chrono::Local);

        writeln!(
            buf,
            "[{} {} {}] {}",
            local_time.format("%H:%M:%S%.3f"),
            record.level(),
            record.module_path().unwrap_or("<unknown>"),
            record.args()
        )
    })
    .init();
}

// fn play_audio_with_cpal() -> Result<()> {
//     use crate::audio::{get_audio_data, play_audio_samples};

//     // Step 1: Decode audio data using symphonia
//     let audio_path = "assets/sfx/Tree Gusts/WINDGust_Wind, Gust in Trees 01_SARM_Wind.wav";
//     let (samples, sample_rate) = get_audio_data(audio_path)?;

//     // Step 2: Play audio data using cpal
//     play_audio_samples(samples, sample_rate)?;

//     Ok(())
// }

pub fn main() {
    // backtrace_on();

    init_env_logger();

    let options = AppOptions::from_args();
    let mut app = AppController::new(options);
    let event_loop = EventLoop::builder().build().unwrap();
    let result = event_loop.run_app(&mut app);

    match result {
        Ok(_) => log::info!("Application exited successfully"),
        Err(e) => log::error!("Application exited with error: {:?}", e),
    }
}
