mod jit;
mod config;
mod translate;
mod context;
mod types;
mod constants;

use std::any::Any;
use std::collections::HashMap;
use std::io::Write as _;

use cranelift::codegen::verifier::VerifierErrors;

use crate::config::Config;



fn main() {
    let config = Config {
        bandwidth_size: 16,
        subgroup_width: 4,
    };

    let pipeline_result = crate::jit::jit_compile("
        struct S {
            x: f32,
            y: f32,
        }

        override blockSize = 16.0;

        // const s : S = S { x: 2.0, y: 3.0 };
        // const h = array(2.0, 3.0, 3.0, 4.0);
        // const d : f32 = 4.0/2.0 + h[0];
        const s: S = S(1, 2);
        const c: f32 = 5;
        const d: f32 = 6;
        const e: i32 = 6;

        // @id(1200) override specular_param: f32 = d;

        // @group(0) @binding(0)
        // var<storage, read> input: array<u32>;

        @group(0) @binding(0)
        var<storage, read_write> output: array<f32, 16>;

        // var<workgroup> x: array<f32, 16>;

        // fn foo(a: array<S, 4>) -> i32 {
        //     // let b = array(S(1, 2), S(3, 4), S(5, 6), S(7, 8));
        //     // a[0].x = 3;

        //     return (*a)[0].x;
        // }

        @compute
        @workgroup_size(1)
        fn main(@builtin(local_invocation_index) thread_id: u32) {
            // var a = 3.0;

            // var b: i32 = 0;
            // a = 5.0;

            var a = 3.0; // + output[thread_id];

            // a = 4.0;
            // a += d;
            output[thread_id] = a;

            // foo(array(S(1, 2), S(3, 4), S(5, 6), S(7, 8)));
            // let a = array(S(1, 2), S(3, 4), S(5, 6), S(7, 8));
            // foo(&a[0]);

            // output[thread_id].x = s.y * i32(thread_id);
            // output[thread_id].y = 3.0;
            // input[2] * 3.0 * f32(blockSize) * specular_param;
        }
    ", &config);

    let pipeline = match pipeline_result {
        Ok(pipeline) => pipeline,
        Err(err) => {
            if let Some(VerifierErrors(errors)) = err.downcast_ref::<VerifierErrors>() {
                for err in errors {
                    eprintln!("Error: {:?}", err);
                }
            } else {
                eprintln!("Error: {:?}", err);
            }

            return;
        }
    };

    let mut output_buffer = vec![0f32; 64];

    let bind_groups: &jit::BindGroups = &[
        jit::BindGroup { entries: &[output_buffer.as_mut_slice().into()] },
    ];

    pipeline.run(8, bind_groups);

    // eprintln!("{:#?}", pipeline);
    eprintln!("{:?}", output_buffer);


/*     let module = naga::front::wgsl::parse_str("
        @group(0) @binding(0)
        var<storage, read> input: array<f32>;

        @group(0) @binding(1)
        var<storage, read_write> output: array<f32>;

        fn main() {
            // let a = 2;
            output[0] = input[2] * 3.0;
        }
    ").unwrap();

    eprintln!("{:#?}", module);

    let module_info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::default(), naga::valid::Capabilities::default() | naga::valid::Capabilities::FLOAT64,
    ).validate(&module).unwrap();

    // eprintln!("{:#?}", module.functions.iter().next().unwrap().1);
    let target = module.functions.iter().next().unwrap();

    // eprintln!("{:#?}", module_info[target.0]);

    let (func, func_sig) = translate_func(&module, &module_info, target.1, &module_info[target.0], &config);

    // let entry_point = &module.entry_points[0];


    let mut flag_builder = settings::builder();

    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    flag_builder.set("is_pic", "false").unwrap();

    let isa_builder = cranelift::native::builder().unwrap();

    let flags = settings::Flags::new(flag_builder);
    let result = codegen::verify_function(&func, &flags);

    eprintln!("{:?}", result);

    // let isa = isa_builder.finish(flags).unwrap();
    // eprintln!("{:?}", isa.pointer_type());


    // let mut jit_module = JITModule::new(JITBuilder::with_isa(isa.clone(), cranelift::module::default_libcall_names()));

    // let mut ctx = jit_module.make_context();
    // let mut func_ctx = FunctionBuilderContext::new();

    // let mut func_ctx = codegen::Context::for_function(func.clone());

    // let func_id = jit_module
    //     .declare_function("a", cranelift::module::Linkage::Local, &func_sig)
    //     .unwrap();

    // jit_module.define_function(func_id, &mut func_ctx).unwrap();
    // jit_module.finalize_definitions().unwrap();

    // let code = jit_module.get_finalized_function(func_id);
    // let func = unsafe { std::mem::transmute::<_, fn(f32, f32, f32, f32) -> (f32, f32, f32, f32)>(code) };

    // // eprintln!("{:?}", code);
    // eprintln!("{:?}", func(2.0, 3.0, 4.0, 5.0));


    // let mut module2 = cranelift_object::ObjectModule::new(cranelift_object::ObjectBuilder::new(
    //     isa,
    //     "foo",
    //     cranelift::module::default_libcall_names(),
    //     // cranelift_object::ObjectBuilder::default_libcall_names(),
    // ).unwrap());

    // let mut func_ctx = codegen::Context::for_function(func.clone());

    // let func_id = module2
    //     .declare_function("a", cranelift::module::Linkage::Local, &func_sig)
    //     .unwrap();

    // let res = module2.define_function(func_id, &mut func_ctx);
    // res.unwrap();

    // let product = module2.finish();
    // let output = product.emit().unwrap();

    // let mut file = std::fs::File::create("output.o").unwrap();
    // file.write_all(&output).unwrap();


    let input_data = (0..4).map(|x| x as f32).collect::<Vec<_>>();
    let mut output_data = vec![0.0; 4];

    let input_buffer: &[u8] = bytemuck::cast_slice(&input_data);
    let output_buffer: &mut [u8] = bytemuck::cast_slice_mut(&mut output_data);

    let buffers = vec![input_buffer, output_buffer]; */
}
