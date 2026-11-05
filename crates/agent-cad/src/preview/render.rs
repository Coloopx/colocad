use super::cache::Part;
use super::*;
const COLORS: [[f64; 3]; 6] = [
    [0.32, 0.68, 0.82],
    [0.40, 0.78, 0.58],
    [0.91, 0.65, 0.30],
    [0.67, 0.52, 0.85],
    [0.88, 0.45, 0.43],
    [0.66, 0.72, 0.78],
];
fn v(p: solidcore_record::Vec3) -> Vec3 {
    Vec3::new(p.x, p.y, p.z)
}
fn unit(x: Vec3) -> Vec3 {
    x / x.length().max(1e-30)
}
fn f32s(out: &mut Vec<u8>, values: impl IntoIterator<Item = f64>) {
    for x in values {
        out.extend_from_slice(&(x as f32).to_le_bytes());
    }
}
fn u32s(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for x in values {
        out.extend_from_slice(&x.to_le_bytes());
    }
}
fn glb(meshes: &[Part<'_>]) -> Result<Vec<u8>> {
    let mut data = vec![];
    let mut views = vec![];
    let mut accessors = vec![];
    let mut objects = vec![];
    let mut nodes = vec![];
    let mut materials = vec![];
    let mut buffers = BTreeMap::new();
    for (i, part) in meshes.iter().enumerate() {
        let name = part.id;
        let m = part.mesh;
        let identity = std::ptr::from_ref(m) as usize;
        let (position,normal,indices)=*buffers.entry(identity).or_insert_with(||{
        let start = data.len();
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &m.positions {
            let p = [p.x, p.y, p.z];
            for j in 0..3 {
                lo[j] = lo[j].min(p[j]);
                hi[j] = hi[j].max(p[j]);
            }
            f32s(&mut data, p);
        }
        views.push(
            json!({"buffer":0,"byteOffset":start,"byteLength":data.len()-start,"target":34962}),
        );
        let position = accessors.len();
        accessors.push(json!({"bufferView":views.len()-1,"componentType":5126,"count":m.positions.len(),"type":"VEC3","min":lo,"max":hi}));
        let start = data.len();
        for p in &m.normals {
            f32s(&mut data, [p.x, p.y, p.z]);
        }
        views.push(
            json!({"buffer":0,"byteOffset":start,"byteLength":data.len()-start,"target":34962}),
        );
        let normal = accessors.len();
        accessors.push(json!({"bufferView":views.len()-1,"componentType":5126,"count":m.normals.len(),"type":"VEC3"}));
        let start = data.len();
        for t in &m.triangles {
            u32s(&mut data, *t);
        }
        views.push(
            json!({"buffer":0,"byteOffset":start,"byteLength":data.len()-start,"target":34963}),
        );
        let indices = accessors.len();
        accessors.push(json!({"bufferView":views.len()-1,"componentType":5125,"count":m.triangles.len()*3,"type":"SCALAR"}));
            (position,normal,indices)
        });
        objects.push(json!({"name":name,"primitives":[{"attributes":{"POSITION":position,"NORMAL":normal},"indices":indices,"material":i}]}));
        let p = part.pose;
        let [x, y, z] = p.axes;
        nodes.push(json!({"name":name,"mesh":i,"matrix":[x.x,x.y,x.z,0.,y.x,y.y,y.z,0.,z.x,z.y,z.z,0.,p.origin.x,p.origin.y,p.origin.z,1.]}));
        let color = COLORS[i % COLORS.len()];
        materials.push(json!({"pbrMetallicRoughness":{"baseColorFactor":[color[0],color[1],color[2],1.],"metallicFactor":0.,"roughnessFactor":0.65},"doubleSided":false}));
    }
    let n = nodes.len();
    nodes.push(json!({"name":"CAD mm Z-up to glTF metre Y-up","children":(0..n).collect::<Vec<_>>(),"scale":[0.001,0.001,0.001],"rotation":[-std::f64::consts::FRAC_1_SQRT_2,0.,0.,std::f64::consts::FRAC_1_SQRT_2]}));
    let doc = json!({"asset":{"version":"2.0","generator":"agent-cad native preview"},"scene":0,"scenes":[{"nodes":[n]}],"nodes":nodes,"meshes":objects,"materials":materials,"buffers":[{"byteLength":data.len()}],"bufferViews":views,"accessors":accessors});
    let mut header = serde_json::to_vec(&doc).map_err(error)?;
    while header.len() % 4 != 0 {
        header.push(b' ');
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let mut out = b"glTF".to_vec();
    u32s(
        &mut out,
        [
            2,
            (12 + 8 + header.len() + 8 + data.len()) as u32,
            header.len() as u32,
            0x4E4F534A,
        ],
    );
    out.extend(header);
    u32s(&mut out, [data.len() as u32, 0x004E4942]);
    out.extend(data);
    Ok(out)
}
fn stl(meshes: &[Part<'_>]) -> Vec<u8> {
    let mut out = vec![0u8; 80];
    let label = b"agent-cad; units=mm; display tessellation";
    out[..label.len()].copy_from_slice(label);
    u32s(
        &mut out,
        [meshes.iter().map(|p| p.mesh.triangles.len() as u32).sum()],
    );
    for part in meshes {
        let m = part.mesh;
        for t in &m.triangles {
            let [a, b, c] = t.map(|i| part.pose.point(v(m.positions[i as usize])));
            let n = unit((b - a).cross(c - a));
            for p in [n, a, b, c] {
                f32s(&mut out, [p.x, p.y, p.z]);
            }
            out.extend_from_slice(&0u16.to_le_bytes());
        }
    }
    out
}
fn png(meshes: &[Part<'_>], o: &PreviewOptions) -> Result<(Vec<u8>, f64, f64)> {
    let start = Instant::now();
    let forward = unit(match o.view {
        View::Iso => Vec3::new(1., -1., 0.8),
        View::Front => -Vec3::Y,
        View::Back => Vec3::Y,
        View::Left => -Vec3::X,
        View::Right => Vec3::X,
        View::Top => Vec3::Z,
        View::Bottom => -Vec3::Z,
    });
    let seed = if forward.z.abs() > 0.99 {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let right = unit(seed.cross(forward));
    let up = forward.cross(right);
    let (w, h) = (o.width as usize, o.height as usize);
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for part in meshes {
        let m = part.mesh;
        for p in &m.positions {
            let p = part.pose.point(v(*p));
            let a = [p.dot(right), p.dot(up)];
            for j in 0..2 {
                lo[j] = lo[j].min(a[j]);
                hi[j] = hi[j].max(a[j]);
            }
        }
    }
    let scale = (w as f64 * 0.88 / (hi[0] - lo[0]).max(1e-9))
        .min(h as f64 * 0.88 / (hi[1] - lo[1]).max(1e-9));
    let cx = (lo[0] + hi[0]) * 0.5;
    let cy = (lo[1] + hi[1]) * 0.5;
    let mut pixels = [13, 22, 35].repeat(w * h);
    let mut depth = vec![f64::NEG_INFINITY; w * h];
    let light = unit(Vec3::new(0.3, -0.4, 0.85));
    for (index, part) in meshes.iter().enumerate() {
        let m = part.mesh;
        let projected = m
            .positions
            .iter()
            .map(|p| {
                let p = part.pose.point(v(*p));
                [
                    (p.dot(right) - cx) * scale + w as f64 / 2.,
                    h as f64 / 2. - (p.dot(up) - cy) * scale,
                    p.dot(forward),
                ]
            })
            .collect::<Vec<_>>();
        for t in &m.triangles {
            let [a, b, c] = t.map(|i| projected[i as usize]);
            let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if den.abs() < 1e-10 {
                continue;
            }
            let x0 = a[0].min(b[0]).min(c[0]).floor().max(0.) as usize;
            let x1 = a[0].max(b[0]).max(c[0]).ceil().min(w as f64 - 1.) as usize;
            let y0 = a[1].min(b[1]).min(c[1]).floor().max(0.) as usize;
            let y1 = a[1].max(b[1]).max(c[1]).ceil().min(h as f64 - 1.) as usize;
            let n = t.map(|i| part.pose.vector(v(m.normals[i as usize])));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                    let wa = ((b[1] - c[1]) * (px - c[0]) + (c[0] - b[0]) * (py - c[1])) / den;
                    let wb = ((c[1] - a[1]) * (px - c[0]) + (a[0] - c[0]) * (py - c[1])) / den;
                    let wc = 1. - wa - wb;
                    if wa < 0. || wb < 0. || wc < 0. {
                        continue;
                    }
                    let z = wa * a[2] + wb * b[2] + wc * c[2];
                    let pixel = y * w + x;
                    if z <= depth[pixel] {
                        continue;
                    }
                    depth[pixel] = z;
                    let normal = unit(n[0] * wa + n[1] * wb + n[2] * wc);
                    let shade = 0.35 + 0.65 * normal.dot(light).abs();
                    for j in 0..3 {
                        pixels[pixel * 3 + j] =
                            (255. * COLORS[index % COLORS.len()][j] * shade) as u8;
                    }
                }
            }
        }
    }
    let raster_ms = start.elapsed().as_secs_f64() * 1000.;
    let tick = Instant::now();
    let mut out = vec![];
    {
        let mut enc = png::Encoder::new(&mut out, o.width, o.height);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(error)?;
        writer.write_image_data(&pixels).map_err(error)?;
    }
    Ok((out, raster_ms, tick.elapsed().as_secs_f64() * 1000.))
}
pub fn artifacts(
    meshes: &[Part<'_>],
    options: &PreviewOptions,
) -> Result<(BTreeMap<String, Vec<u8>>, Value)> {
    let clipped;
    let cut_parts;
    let meshes = if let Some(plane) = options.section {
        let world = meshes
            .iter()
            .map(|p| (p.id.to_owned(), super::placed(p.mesh, p.pose)))
            .collect::<Vec<_>>();
        clipped = super::section::clip(&world, plane)?;
        cut_parts = clipped
            .iter()
            .map(|(id, mesh)| Part {
                id,
                mesh,
                pose: Default::default(),
            })
            .collect::<Vec<_>>();
        &cut_parts
    } else {
        meshes
    };
    let (png, raster_ms, png_ms) = png(meshes, options)?;
    let tick = Instant::now();
    let glb = glb(meshes)?;
    let glb_ms = tick.elapsed().as_secs_f64() * 1000.;
    let tick = Instant::now();
    let stl = stl(meshes);
    let stl_ms = tick.elapsed().as_secs_f64() * 1000.;
    let out = BTreeMap::from([
        ("preview.png".into(), png),
        ("model.glb".into(), glb),
        ("model.stl".into(), stl),
    ]);
    if out.values().map(Vec::len).sum::<usize>() > 32 * 1024 * 1024 {
        return Err(CadError::new(
            "PREVIEW_BUDGET",
            "display artifacts exceed 32 MiB",
        ));
    }
    Ok((
        out,
        json!({"display_triangles":meshes.iter().map(|p|p.mesh.triangles.len()).sum::<usize>(),"raster":raster_ms,"png_encode":png_ms,"glb_encode":glb_ms,"stl_encode":stl_ms}),
    ))
}
