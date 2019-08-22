#[macro_use]
extern crate criterion;

use criterion::black_box;
use criterion::Criterion;

use j4rs::{self, Instance, InvocationArg, Jvm};

fn do_instance_creation(jvm: Jvm) -> Instance {
    jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap()
}

fn do_invocation_w_no_args(jvm: Jvm, instance: Instance) -> Instance {
    jvm.invoke(&instance, "getMyString", &[]).unwrap()
}

fn do_invocation_w_args(jvm: Jvm, instance: Instance) -> Instance {
    jvm.invoke(&instance, "getMyWithArgs", &vec![InvocationArg::from("a")]).unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(
        "instances creation",
        |b| b.iter(|| {
            let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
            do_instance_creation(black_box(jvm))
        }));

    c.bench_function(
        "invocations with no args and String result",
        |b| b.iter(|| {
            let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
            let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
            do_invocation_w_no_args(black_box(jvm), black_box(instance))
        }));

    c.bench_function(
        "invocations with String arg and String result",
        |b| b.iter(|| {
            let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
            let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
            do_invocation_w_args(black_box(jvm), black_box(instance))
        }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);