//! Inspect template-free native geometry. The probe never publishes an editable part.
use cadmpeg_codec_sldprt::SldprtCodec;
use cadmpeg_codec_step::StepCodec;
use cadmpeg_ir::codec::write::{target::TargetRequest, EncodeInput, Encoder};
use cadmpeg_ir::{validate::validate_neutral_with_source_fidelity, CadIr, Codec, DecodeOptions};
use std::{env, error::Error, fs::File, io::Cursor};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let options = DecodeOptions::default();
    match args.as_slice() {
        [op, file] if op == "read-step" => {
            let decoded = StepCodec::default().decode(&mut File::open(file)?, &options)?;
            serde_json::to_writer(std::io::stdout().lock(), decoded.ir())?;
        }
        [op, file, carrier] if op == "probe" => {
            let mut ir: CadIr = serde_json::from_reader(File::open(file)?)?;
            let (blank, _, fidelity) = SldprtCodec.decode(&mut File::open(carrier)?, &options)?.into_parts();
            ir.source = blank.source;
            // This graph is newly authored. The empty envelope supplies only
            // framing, so its empty history has no relationship to this graph.
            if let Some(source) = ir.source.as_mut() {
                source.attributes.clear();
            }
            ir.finalize();
            let mut encoded = Vec::new();
            SldprtCodec.plan(EncodeInput::new(&ir, Some(&fidelity)), TargetRequest::Inherit)?.write_to(&mut encoded)?;
            let (roundtrip, _, reader_source) = SldprtCodec.decode(&mut Cursor::new(&encoded), &options)?.into_parts();
            let checks=validate_neutral_with_source_fidelity(&roundtrip,&reader_source,vec![])?;
            let native=serde_json::to_value(&roundtrip.native)?;
            let m=roundtrip.model;
            serde_json::to_writer_pretty(std::io::stdout().lock(),&serde_json::json!({
                "experimentalContainerBytes":encoded.len(),"solids":m.bodies.len(),"faces":m.faces.len(),"edges":m.edges.len(),
                "features":m.features.len(),"sketches":m.sketches.len(),"checks":checks.findings,
                "history":m.features.iter().map(|f|serde_json::json!({"name":f.name,"definition":f.evaluation.definition(),"dependencies":f.dependencies})).collect::<Vec<_>>(),
                "nativeOperationClasses":native["sldprt"]["feature_input_classes"].as_array().map(Vec::len),
                "drivingScalarRecords":native["sldprt"]["feature_input_scalars"].as_array().map(Vec::len),
                "editablePartVerified":false
            }))?;
        }
        _ => return Err("expected: read-step model.step | probe equivalent-ir.json empty-envelope.bin".into())
    }
    Ok(())
}
