use mlua::prelude::LuaValue;
use mlua::{AnyUserData, Lua, MetaMethod, UserData, UserDataFields, UserDataMethods};
use parking_lot::Mutex;
use rust_htslib::bcf;
use rust_htslib::bcf::record::Buffer;
use std::sync::Arc;

struct SBuffer(bcf::record::BufferBacked<'static, Vec<&'static [i32]>, Buffer>);

unsafe impl Send for SBuffer {}
impl UserData for SBuffer {}

fn register_variant(lua: &Lua) -> mlua::Result<()> {
    lua.register_userdata_type::<Arc<Mutex<SBuffer>>>(|reg| {
        reg.add_meta_function(
            MetaMethod::Index,
            |_lua, (this, idx): (AnyUserData, usize)| {
                eprintln!("indexing into SBuffer index: {:?}", idx);
                this.borrow::<Arc<Mutex<SBuffer>>>()?
                    .lock()
                    .0
                    .get(idx - 1)
                    // TODO: make this get phased and allele with >> 1 - 1 and & 1.
                    .map(|&x| x.iter().copied().collect::<Vec<i32>>())
                    .ok_or_else(|| {
                        let msg = format!("index out of bounds");
                        mlua::Error::RuntimeError(msg)
                    })
            },
        );
        reg.add_meta_function(MetaMethod::Len, |_lua, this: AnyUserData| {
            let len = this.borrow::<Arc<Mutex<SBuffer>>>()?.lock().0.len();
            Ok(len)
        });
    })?;
    lua.register_userdata_type::<bcf::Record>(|reg| {
        reg.add_meta_function(
            MetaMethod::Index,
            |_lua, (_, name): (AnyUserData, String)| {
                let msg = format!("field '{:?}' variant.{:?} not found", name, name);
                Err::<LuaValue<'_>, mlua::Error>(mlua::Error::RuntimeError(msg))
            },
        );
        reg.add_field_method_get("id", |lua: &Lua, this: &bcf::Record| {
            lua.create_string(this.id())
        });

        reg.add_field_method_set(
            "id",
            |_lua: &Lua, this: &mut bcf::Record, new_id: String| {
                // Q: how can I make this work?
                match this.set_id(new_id.as_bytes()) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(mlua::Error::RuntimeError(e.to_string())),
                }
            },
        );

        reg.add_field_method_get("genotypes", |_lua: &Lua, this: &bcf::Record| {
            let genotypes = this.format(b"GT");
            match genotypes.integer() {
                Ok(genotypes) => {
                    let sb = Arc::new(Mutex::new(SBuffer(genotypes)));
                    Ok(sb)
                }
                Err(e) => Err(mlua::Error::RuntimeError(e.to_string())),
            }
        });
    })
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    register_variant(&lua).expect("error registering variant");

    let mut header = bcf::Header::new();
    header.push_record(r#"##contig=<ID=chr1,length=10000>"#.as_bytes());
    header
        .push_record(r#"##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">"#.as_bytes());
    header.push_sample("NA12878".as_bytes());
    header.push_sample("NA12879".as_bytes());
    let mut vcf = bcf::Writer::from_stdout(&header, true, bcf::Format::Vcf).unwrap();
    let mut record = vcf.empty_record();
    let _ = record.set_rid(Some(vcf.header().name2rid(b"chr1").unwrap()));
    record.set_pos(6);
    record.set_id(b"rs1234")?;
    let alleles = &[
        bcf::record::GenotypeAllele::Unphased(0),
        bcf::record::GenotypeAllele::Phased(1),
        bcf::record::GenotypeAllele::Unphased(1),
        bcf::record::GenotypeAllele::Unphased(1),
    ];
    record.push_genotypes(alleles)?;

    let get_expression = r#"return variant.id"#;
    let set_expression = r#"variant.id = 'rsabcd'"#;
    let gts_expr = r#"local gts = variant.genotypes; 
    for i = 1, #gts do print(gts[i][1]) end
    "#;
    let get_exp = lua
        .load(get_expression)
        .set_name("get")
        .into_function()
        .unwrap();
    let set_exp = lua
        .load(set_expression)
        .set_name("set")
        .into_function()
        .unwrap();
    let gts_exp = lua.load(gts_expr).set_name("gts").into_function().unwrap();
    let globals = lua.globals();
    vcf.write(&record).unwrap();

    lua.scope(|scope| {
        let ud = scope.create_any_userdata_ref(&record)?;
        globals.raw_set("variant", ud)?;
        let result = get_exp.call::<_, String>(())?;
        eprintln!("result of getter: {}", result);

        // error here with setting variant.id
        let result = set_exp.call::<_, ()>(())?;
        eprintln!("result of setter: {:?}", result);

        let result = gts_exp.call::<_, ()>(())?;
        eprintln!("result of gts: {:?}", result);

        Ok(())
    })
    .expect("error in scope");

    vcf.write(&record).unwrap();

    Ok(())
}
