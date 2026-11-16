//! [INPUT]: 公开occt-drilled-block.step及自有17度seam换基。
//! [OUTPUT]: 运行时正例STEP和单顶点不一致的负例；本机不生成文件。
//! [POS]: 完整实体交付回归的固定夹具，不改生产建模/导入策略。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
pub const REFERENCE_SHA256: &str = "576b003a1788fce49a3512f683aa8e1463e808fbcbd0bf6454f5c1ace7435675";
pub const REFERENCE: &str = include_str!("occt-drilled-block.step");

fn replace_record(step: &mut String, id: u64, record: &str) {
    let marker = format!("#{id} =");
    let start = step.find(&marker).expect("fixed public fixture record");
    let end = start + step[start..].find(';').unwrap() + 1;
    step.replace_range(start..end, &format!("#{id} = {record};"));
}

pub fn rotated_seam() -> String {
    let mut step = REFERENCE.to_string();
    let (s, c) = 17_f64.to_radians().sin_cos();
    let (x, y) = (10. + 2. * c, 8. + 2. * s);
    // 两圆、圆柱的零角轴同步旋转；三维曲面集合不因换基而改变。
    for id in [273, 286, 380] {
        replace_record(&mut step, id, &format!("DIRECTION('',({c:.17E},{s:.17E},0.))"));
    }
    for id in [279, 386] {
        replace_record(&mut step, id, &format!("DIRECTION('',({c:.17E},{s:.17E}))"));
    }
    // 3D seam端点与纵缝起点同步；圆柱pcurve保留相对0和2pi两份。
    for (id, z) in [(267, 10.), (374, 0.), (410, 0.)] {
        replace_record(&mut step, id, &format!("CARTESIAN_POINT('',({x:.17E},{y:.17E},{z:.17E}))"));
    }
    step
}

pub fn inconsistent_seam_vertex() -> String {
    let mut step = rotated_seam();
    let (s, c) = 17_f64.to_radians().sin_cos();
    // 只改底顶点Z；不改圆、平面或纵缝，构造0.25mm真实几何不一致。
    replace_record(&mut step, 374, &format!("CARTESIAN_POINT('',({:.17E},{:.17E},0.25))", 10. + 2. * c, 8. + 2. * s));
    step
}
