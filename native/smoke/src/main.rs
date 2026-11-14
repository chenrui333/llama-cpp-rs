//! Small real inference consumer, rebuilt in external mode before release.
#[cfg(test)]
#[path = "../../../llama-cpp-sys-2/build_support/artifacts.rs"]
mod artifact_contract;
#[cfg(test)]
#[allow(dead_code, unexpected_cfgs)]
#[path = "../../../llama-cpp-sys-2/build_support/external.rs"]
mod external_contract;
#[cfg(test)]
use external_contract as external;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../llama-cpp-sys-2/build_support/automatic.rs"]
mod automatic_contract;
#[cfg(test)]
use automatic_contract as automatic;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../../llama-cpp-sys-2/build_support/source.rs"]
mod source_contract;
#[cfg(test)]
mod source_tests;
#[cfg(test)]
mod automatic_tests;
#[cfg(test)]
fn macos_link_search_path() -> Option<String> {
    None
}
#[cfg(test)]
mod contract_tests;
use llama_cpp_2::{
    context::params::{LlamaContextParams, LlamaPoolingType},
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaModel},
};
use std::{
    ffi::{CStr, CString},
    num::NonZeroU32,
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: native-smoke BUNDLE MODEL cpu|cuda13|cuda12|metal".into());
    }
    let bundle = PathBuf::from(&args[1]);
    let mode = &args[3];
    if !["cpu", "cuda13", "cuda12", "metal"].contains(&mode.as_str()) {
        return Err("unknown backend".into());
    }
    #[cfg(feature = "cuda")]
    {
        // Match zg's primary-then-fallback loading, but require the requested
        // CUDA generation to actually load for this test.
        llama_cpp_2::llama_backend::load_backends_from_path(&bundle.join("lib"));
        if mode == "cuda12" {
            let name = if cfg!(windows) {
                "ggml-cuda.dll"
            } else {
                "libggml-cuda.so"
            };
            let path = CString::new(
                bundle
                    .join("lib/cuda12")
                    .join(name)
                    .to_str()
                    .ok_or("non UTF-8 path")?,
            )?;
            if unsafe { llama_cpp_sys_2::ggml_backend_load(path.as_ptr()) }.is_null() {
                return Err("CUDA 12 plugin did not load".into());
            }
        }
    }
    let _ = &bundle;
    // Exercise the packaged C++ wrapper as well as core inference. Otherwise
    // an unused static wrapper could hide missing common libraries at link time.
    let schema = CString::new(r#"{"type":"string"}"#)?;
    let mut grammar = std::ptr::null_mut();
    let status = unsafe {
        llama_cpp_sys_2::llama_rs_json_schema_to_grammar(schema.as_ptr(), false, &mut grammar)
    };
    if status != llama_cpp_sys_2::LLAMA_RS_STATUS_OK || grammar.is_null() {
        return Err("common wrapper schema conversion failed".into());
    }
    let valid_grammar = !unsafe { CStr::from_ptr(grammar) }.to_bytes().is_empty();
    unsafe { llama_cpp_sys_2::llama_rs_string_free(grammar) };
    assert!(valid_grammar);
    let backend = LlamaBackend::init()?;
    let mut gpu = None;
    for index in 0..unsafe { llama_cpp_sys_2::ggml_backend_dev_count() } {
        let device = unsafe { llama_cpp_sys_2::ggml_backend_dev_get(index) };
        let name =
            unsafe { CStr::from_ptr(llama_cpp_sys_2::ggml_backend_dev_name(device)) }.to_str()?;
        eprintln!("available device: {index} {name}");
        if (mode.starts_with("cuda") && name.starts_with("CUDA"))
            || (mode == "metal" && (name.starts_with("Metal") || name.starts_with("MTL")))
        {
            gpu = Some(index);
        }
    }
    let mut params = LlamaModelParams::default().with_n_gpu_layers(0);
    if mode != "cpu" {
        params = params
            .with_n_gpu_layers(1000)
            .with_devices(&[gpu.ok_or("required GPU device not found")?])?;
    }
    let model = LlamaModel::load_from_file(&backend, &args[2], &params)?;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(512))
        .with_n_batch(512)
        .with_n_ubatch(512)
        .with_n_threads(2)
        .with_n_threads_batch(2)
        .with_embeddings(true)
        .with_pooling_type(LlamaPoolingType::Mean)
        .with_offload_kqv(mode != "cpu")
        .with_op_offload(mode != "cpu");
    let mut context = model.new_context(&backend, ctx_params)?;
    let tokens = model.str_to_token(
        "task: search result | query: native runtime compatibility",
        AddBos::Always,
    )?;
    let mut batch = LlamaBatch::new(512, 1);
    batch.add_sequence(&tokens, 0, false)?;
    context.decode(&mut batch)?;
    let embedding = context.embeddings_seq_ith(0)?;
    assert_eq!(embedding.len(), 768, "EmbeddingGemma embedding dimension");
    assert!(embedding.iter().all(|v| v.is_finite()));
    let norm = embedding
        .iter()
        .map(|v| f64::from(*v).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(norm > 0.0);
    println!(
        "{}",
        serde_json::json!({"mode": mode, "dimension": embedding.len(), "norm": norm, "embedding": embedding})
    );
    Ok(())
}
