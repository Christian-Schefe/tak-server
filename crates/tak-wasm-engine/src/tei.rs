use gloo_timers::future::TimeoutFuture;
use tiltak::tei;
use wasm_bindgen_futures::spawn_local;

use crate::performance_now;

struct WasmPlatform;

impl tei::Platform for WasmPlatform {
    type Instant = f64;

    fn yield_fn() -> impl std::future::Future {
        TimeoutFuture::new(0)
    }

    fn current_time() -> Self::Instant {
        performance_now()
    }

    fn elapsed_time(start: &Self::Instant) -> std::time::Duration {
        std::time::Duration::from_millis((performance_now() - start) as u64)
    }
}

pub fn run<Out: Fn(&str) + 'static>(output_callback: Out) -> async_channel::Sender<String> {
    let (sender, receiver) = async_channel::unbounded();

    spawn_local(async move {
        tei::tei::<_, WasmPlatform>(false, false, receiver, &output_callback).await
    });

    sender
}
