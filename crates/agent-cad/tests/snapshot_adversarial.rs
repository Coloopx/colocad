//! Deterministic parser probes with valid recomputed metadata hashes: these reach
//! the native parser rather than stopping at the public integrity check.
use agent_cad::query::snapshot::DefinitionSnapshot;
use sha2::{Digest, Sha256};
use solidcore::{Kernel, Vec3};
#[test]
fn truncated_and_invalid_native_payloads_fail_without_panicking() {
    let mut kernel=Kernel::default();
    let body=solidcore::primitive::api_primitives::create_box(&mut kernel,Vec3::ZERO,20.,30.,40.).unwrap();
    let source=DefinitionSnapshot::capture(&kernel,&body).unwrap();
    let mut lengths=vec![0,1,2,4,8,16,32,64,128,256,source.geometry.len()/2,source.geometry.len()-1];
    lengths.sort();lengths.dedup();
    for length in lengths {
        let mut bad=source.clone();bad.geometry.truncate(length);bad.geometry_sha256=format!("{:x}",Sha256::digest(&bad.geometry));let expected=bad.digest();
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||bad.load(&mut Kernel::default(),&expected)));
        assert!(result.is_ok(),"native parser panicked at length {length}");assert!(result.unwrap().is_err(),"truncated payload accepted at {length}");
    }
    for byte in [0,0xff,b'X'] {
        let mut bad=source.clone();bad.geometry[..8].fill(byte);bad.geometry_sha256=format!("{:x}",Sha256::digest(&bad.geometry));let expected=bad.digest();
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||bad.load(&mut Kernel::default(),&expected)));
        assert!(result.is_ok(),"invalid header panicked");assert!(result.unwrap().is_err(),"invalid header accepted");
    }
}
