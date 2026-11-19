//! [INPUT]: 原生简单盒体、Part21公开语法与测试给定的单位/轴系。
//! [OUTPUT]: 带明确body context、CDSR和mapped item的合成STEP。
//! [POS]: 仅供step_units回归；不复制生产单位或位姿推导算法。
//! [PROTOCOL]: 变更时更新此头部，然后检查 AGENTS.md
use solidcore::{Kernel, Vec3};
use std::collections::BTreeMap;
use step_import::part21::{self, Instance, Param, Record};

pub struct BoxIds {
    pub body: u64,
    pub rep: u64,
    pub product: u64,
}
pub struct Fixture {
    pub entities: BTreeMap<u64, Instance>,
    next: u64,
}
fn representation(item: &Instance) -> &Record {
    records(item)
        .iter()
        .find(|r| r.name.ends_with("REPRESENTATION"))
        .unwrap()
}
fn records(item: &Instance) -> &[Record] {
    match item {
        Instance::Simple(r) => std::slice::from_ref(r),
        Instance::Complex(rs) => rs,
    }
}
fn parameter(p: &Param, offset: u64) -> String {
    match p {
        Param::Int(n) => n.to_string(),
        Param::Real(n) => format!("{n:.17E}"),
        Param::Str(s) => format!("'{}'", s.replace('\'', "''")),
        Param::Enum(s) => format!(".{s}."),
        Param::Binary(s) => format!("\"{s}\""),
        Param::Ref(id) => format!("#{}", id + offset),
        Param::Null => "$".into(),
        Param::Derived => "*".into(),
        Param::List(v) => format!(
            "({})",
            v.iter()
                .map(|p| parameter(p, offset))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Param::Typed(n, p) => format!("{n}({})", parameter(p, offset)),
    }
}
fn instance(item: &Instance, offset: u64) -> String {
    let rs = records(item)
        .iter()
        .map(|r| {
            format!(
                "{}({})",
                r.name,
                r.params
                    .iter()
                    .map(|p| parameter(p, offset))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>();
    match item {
        Instance::Simple(_) => rs[0].clone(),
        Instance::Complex(_) => format!("({})", rs.join(" ")),
    }
}
impl Fixture {
    pub fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
            next: 1,
        }
    }
    pub fn add(&mut self, record: &str) -> u64 {
        let id = self.next;
        let text =
            format!("ISO-10303-21;HEADER;ENDSEC;DATA;#{id}={record};ENDSEC;END-ISO-10303-21;");
        let parsed = part21::parse(text.as_bytes(), &Default::default()).unwrap();
        self.entities.insert(id, parsed.entities[&id].clone());
        self.next += 1;
        id
    }
    pub fn context(&mut self, cm: bool, degrees: bool) -> u64 {
        let length = self.add(&format!(
            "(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.{}.,.METRE.))",
            if cm { "CENTI" } else { "MILLI" }
        ));
        let rad = self.add("(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))");
        let angle = if degrees {
            let factor = self.add(&format!(
                "PLANE_ANGLE_MEASURE_WITH_UNIT(PLANE_ANGLE_MEASURE(1.7453292519943295E-2),#{rad})"
            ));
            self.add(&format!(
                "(CONVERSION_BASED_UNIT('DEGREE',#{factor}) NAMED_UNIT(*) PLANE_ANGLE_UNIT())"
            ))
        } else {
            rad
        };
        self.add(&format!("(GEOMETRIC_REPRESENTATION_CONTEXT(3) GLOBAL_UNIT_ASSIGNED_CONTEXT((#{length},#{angle})) REPRESENTATION_CONTEXT('',''))"))
    }
    pub fn box_body(&mut self, cm: bool, degrees: bool) -> BoxIds {
        let mut k = Kernel::default();
        let b = solidcore::primitive::api_primitives::create_box(&mut k, Vec3::ZERO, 2., 3., 4.)
            .unwrap();
        let text = step_export::write(&step_brep::brep_to_model(&b).unwrap()).unwrap();
        let parsed = part21::parse(text.as_bytes(), &Default::default()).unwrap();
        let offset = self.next;
        let find = |name| {
            *parsed
                .entities
                .iter()
                .find(|(_, v)| v.record(name).is_some())
                .unwrap()
                .0
                + offset
        };
        let ids = BoxIds {
            body: find("MANIFOLD_SOLID_BREP"),
            rep: find("ADVANCED_BREP_SHAPE_REPRESENTATION"),
            product: find("PRODUCT_DEFINITION"),
        };
        let records = parsed
            .entities
            .iter()
            .map(|(id, item)| format!("#{}={};", id + offset, instance(item, offset)))
            .collect::<String>();
        let text = format!("ISO-10303-21;HEADER;ENDSEC;DATA;{records}ENDSEC;END-ISO-10303-21;");
        let shifted = part21::parse(text.as_bytes(), &Default::default()).unwrap();
        let length_ids = shifted
            .entities
            .iter()
            .filter(|(_, v)| v.record("LENGTH_UNIT").is_some() && v.record("SI_UNIT").is_some())
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        self.entities.extend(shifted.entities);
        if cm {
            for id in length_ids {
                self.record_mut(id, "SI_UNIT").params[0] = Param::Enum("CENTI".into());
            }
        }
        self.next = self.entities.keys().max().unwrap() + 1;
        let context = self.context(cm, degrees);
        self.record_mut(ids.rep, "ADVANCED_BREP_SHAPE_REPRESENTATION")
            .params[2] = Param::Ref(context);
        ids
    }
    pub fn uncertainty(&mut self, rep: u64, value: f64) {
        let r = representation(&self.entities[&rep]);
        let ctx = r.params[2].as_ref_id().unwrap();
        let length = self.entities[&ctx]
            .record("GLOBAL_UNIT_ASSIGNED_CONTEXT")
            .unwrap()
            .params[0]
            .as_list()
            .unwrap()[0]
            .as_ref_id()
            .unwrap();
        let measure = self.add(&format!(
            "UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE({value:E}),#{length},'distance','')"
        ));
        if let Instance::Complex(rs) = self.entities.get_mut(&ctx).unwrap() {
            rs.push(Record {
                name: "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT".into(),
                params: vec![Param::List(vec![Param::Ref(measure)])],
            });
        } else {
            panic!("complex context required");
        }
    }
    pub fn record_mut(&mut self, id: u64, name: &str) -> &mut Record {
        match self.entities.get_mut(&id).unwrap() {
            Instance::Simple(r) => {
                assert_eq!(r.name, name);
                r
            }
            Instance::Complex(rs) => rs.iter_mut().find(|r| r.name == name).unwrap(),
        }
    }
    pub fn frame(&mut self, p: [f64; 3], turn: u8) -> u64 {
        let point = self.add(&format!("CARTESIAN_POINT('',({},{},{}))", p[0], p[1], p[2]));
        let z = self.add("DIRECTION('',(0.,0.,1.))");
        let x = self.add(match turn % 4 {
            0 => "DIRECTION('',(1.,0.,0.))",
            1 => "DIRECTION('',(0.,1.,0.))",
            2 => "DIRECTION('',(-1.,0.,0.))",
            _ => "DIRECTION('',(0.,-1.,0.))",
        });
        self.add(&format!("AXIS2_PLACEMENT_3D('',#{point},#{z},#{x})"))
    }
    pub fn representation(&mut self, items: &[u64], context: u64) -> u64 {
        self.add(&format!(
            "SHAPE_REPRESENTATION('',({}),#{context})",
            items
                .iter()
                .map(|i| format!("#{i}"))
                .collect::<Vec<_>>()
                .join(",")
        ))
    }
    pub fn product(&mut self, rep: u64) -> u64 {
        let app = self.add("APPLICATION_CONTEXT('mechanical')");
        let pc = self.add(&format!("PRODUCT_CONTEXT('',#{app},'mechanical')"));
        let product = self.add(&format!("PRODUCT('group','group','',(#{pc}))"));
        let formation = self.add(&format!("PRODUCT_DEFINITION_FORMATION('','',#{product})"));
        let dc = self.add(&format!("PRODUCT_DEFINITION_CONTEXT('',#{app},'design')"));
        let pd = self.add(&format!(
            "PRODUCT_DEFINITION('design','',#{formation},#{dc})"
        ));
        let shape = self.add(&format!("PRODUCT_DEFINITION_SHAPE('','',#{pd})"));
        self.add(&format!("SHAPE_DEFINITION_REPRESENTATION(#{shape},#{rep})"));
        pd
    }
    pub fn occurrence(
        &mut self,
        parent: u64,
        child: u64,
        from_rep: u64,
        to_rep: u64,
        from: u64,
        to: u64,
    ) {
        let transform = self.add(&format!("ITEM_DEFINED_TRANSFORMATION('','',#{from},#{to})"));
        let rr = self.add(&format!("(REPRESENTATION_RELATIONSHIP('','',#{from_rep},#{to_rep}) REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{transform}) SHAPE_REPRESENTATION_RELATIONSHIP())"));
        let usage = self.add(&format!(
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE('1','','',#{parent},#{child},$)"
        ));
        let shape = self.add(&format!("PRODUCT_DEFINITION_SHAPE('','',#{usage})"));
        self.add(&format!(
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#{rr},#{shape})"
        ));
    }
    pub fn mapped(&mut self, rep: u64, from: u64, target: [f64; 3], turn: u8) -> u64 {
        let map = self.add(&format!("REPRESENTATION_MAP(#{from},#{rep})"));
        let p = self.add(&format!(
            "CARTESIAN_POINT('',({},{},{}))",
            target[0], target[1], target[2]
        ));
        let (x, y) = match turn % 4 {
            0 => ([1, 0, 0], [0, 1, 0]),
            1 => ([0, 1, 0], [-1, 0, 0]),
            2 => ([-1, 0, 0], [0, -1, 0]),
            _ => ([0, -1, 0], [1, 0, 0]),
        };
        let dx = self.add(&format!("DIRECTION('',({},{},{}))", x[0], x[1], x[2]));
        let dy = self.add(&format!("DIRECTION('',({},{},{}))", y[0], y[1], y[2]));
        let op = self.add(&format!(
            "CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#{dx},#{dy},#{p},1.,$)"
        ));
        self.add(&format!("MAPPED_ITEM('',#{map},#{op})"))
    }
    pub fn text(&self) -> String {
        let mut s = String::from(
            "ISO-10303-21;\nHEADER;FILE_SCHEMA(('CONFIG_CONTROL_DESIGN'));ENDSEC;\nDATA;\n",
        );
        for (id, item) in &self.entities {
            s.push_str(&format!("#{id}={};\n", instance(item, 0)));
        }
        s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        s
    }
    pub fn replace_product_rep(&mut self, product: u64, rep: u64) {
        let shape = *self
            .entities
            .iter()
            .find(|(_, v)| {
                v.record("PRODUCT_DEFINITION_SHAPE")
                    .is_some_and(|r| r.params[2].as_ref_id() == Some(product))
            })
            .unwrap()
            .0;
        let sdr = *self
            .entities
            .iter()
            .find(|(_, v)| {
                v.record("SHAPE_DEFINITION_REPRESENTATION")
                    .is_some_and(|r| r.params[0].as_ref_id() == Some(shape))
            })
            .unwrap()
            .0;
        self.record_mut(sdr, "SHAPE_DEFINITION_REPRESENTATION")
            .params[1] = Param::Ref(rep);
    }
}
