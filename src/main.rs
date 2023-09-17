#![feature(rustc_private, let_chains)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_lint_defs;
extern crate rustc_session;
extern crate rustc_span;

use std::{process, str};

const USAGE: &str = r#"Usage: cargo run <FILE>"#;

fn main() {
    println!("{USAGE}");
    run();
}

struct TestGenContext {
    functions: Vec<Function>,
}

impl TestGenContext {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }

    fn push_function(&mut self, func: Function) {
        self.functions.push(func);
    }

    fn get_functions(&self) -> &Vec<Function> {
        &self.functions
    }

    fn dump(&self) {
        println!("=== DUMP ===");
        for func in self.get_functions() {
            print!("fn {}(", func.name);
            for param in &func.inputs {
                print!("{}: {},", param.name, param.ty.name);
            }
            println!(") -> {} {{}}", func.output.name);
        }
    }
}

struct Function {
    name: String,
    inputs: Vec<Param>,
    output: Ty,
}

struct Param {
    name: String,
    ty: Ty,
}

struct Ty {
    name: String,
}

fn run() {
    let mut ctx = TestGenContext::new();
    // TODO: Use cargo as driver
    //rustc_driver::init_rustc_env_logger();
    let rustc_exit_code = rustc_driver::catch_with_exit_code(move || {
        let out = process::Command::new("rustc")
            .arg("--print=sysroot")
            .current_dir(".")
            .output()
            .unwrap();
        let sys_root = str::from_utf8(&out.stdout).unwrap().trim().to_string();

        let orig_args: Vec<String> = std::env::args().collect();
        let filepath = orig_args.last().unwrap().to_string();

        let args: Vec<String> = vec![
            "rustc".to_string(),
            filepath,
            "--sysroot".to_string(),
            sys_root,
        ];

        rustc_driver::RunCompiler::new(&args, &mut RustcCallBacks { ctx: &mut ctx }).run()
    });

    if rustc_exit_code != 0 {
        println!("rustc exited with code {}", rustc_exit_code);
        std::process::exit(1);
    }
}

struct RustcCallBacks<'ctx> {
    ctx: &'ctx mut TestGenContext,
}

impl rustc_driver::Callbacks for RustcCallBacks<'_> {
    fn config(&mut self, _config: &mut rustc_interface::Config) {}

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> rustc_driver::Compilation {
        queries.global_ctxt().unwrap().enter(|tcx| {
            let mut visitor = FnCollect::new(self.ctx);
            tcx.hir().visit_all_item_likes_in_crate(&mut visitor);
        });
        self.ctx.dump();
        rustc_driver::Compilation::Stop
    }
}

struct FnCollect<'ctx> {
    ctx: &'ctx mut TestGenContext,
}
impl<'ctx> FnCollect<'ctx> {
    fn new(ctx: &'ctx mut TestGenContext) -> Self {
        Self { ctx }
    }

    fn push_function(&mut self, func: Function) {
        self.ctx.push_function(func);
    }
}

impl<'rustc> rustc_hir::intravisit::Visitor<'rustc> for FnCollect<'_> {
    fn visit_item(&mut self, item: &'rustc rustc_hir::Item<'_>) {
        match item.kind {
            rustc_hir::ItemKind::Fn(fnsig, _generics, _body) => {
                let name = item.ident.to_string();
                println!("[rustc] Found {}", name);

                let mut params = vec![];
                for param in fnsig.decl.inputs {
                    params.push(Param {
                        name: "_".to_string(),
                        ty: Ty {
                            name: ty_to_string(param),
                        },
                    });
                }

                let ret = match &fnsig.decl.output {
                    rustc_hir::FnRetTy::DefaultReturn(_) => Ty {
                        name: "()".to_string(),
                    },
                    rustc_hir::FnRetTy::Return(ty) => Ty {
                        name: ty_to_string(ty),
                    },
                };

                self.push_function(Function {
                    name,
                    inputs: params,
                    output: ret,
                });
            }
            _ => {}
        }
    }
}

fn path_to_strinig(path: &rustc_hir::Path<'_>) -> String {
    let mut s = String::new();
    for seg in path.segments {
        s.push_str(seg.ident.as_str());
        s.push_str("::");
    }
    // Remove tailing `::`
    s.pop();
    s.pop();
    s
}

fn ty_to_string(ty: &rustc_hir::Ty<'_>) -> String {
    match ty.kind {
        rustc_hir::TyKind::Path(qpath) => match qpath {
            rustc_hir::QPath::Resolved(_, path) => path_to_strinig(path),
            _ => todo!(),
        },
        _ => todo!(),
    }
}
