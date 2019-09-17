#[macro_use]
extern crate criterion;

use criterion::black_box;
use criterion::Criterion;

use j4rs::{self, Instance, InvocationArg, Jvm};

fn do_instance_creation(jvm: &Jvm) -> Instance {
    jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap()
}

fn do_invocation_w_no_args(jvm: &Jvm, instance: &Instance) -> Instance {
    jvm.invoke(instance, "getMyString", &[]).unwrap()
}

fn do_invocation_w_args(jvm: &Jvm, instance: &Instance) -> Instance {
    jvm.invoke(instance, "getMyWithArgs", &vec![InvocationArg::from("a")]).unwrap()
}

fn do_invocation_w_new_args(jvm: &Jvm, instance: &Instance) -> Instance {
    let ia = InvocationArg::new_2(&"a".to_string(), "java.lang.String", jvm).unwrap();
    jvm.invoke(instance, "getMyWithArgs", &vec![ia]).unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
    c.bench_function(
        "instances creation",
        move |b| b.iter(|| {
            do_instance_creation(black_box(&jvm))
        }));

    let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
    let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
    c.bench_function(
        "invocations with no args and String result",
        move |b| b.iter(|| {
            do_invocation_w_no_args(black_box(&jvm), black_box(&instance))
        }));

    let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
    let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
    c.bench_function(
        "invocations with String arg and String result",
        move |b| b.iter(|| {
            do_invocation_w_args(black_box(&jvm), black_box(&instance))
        }));

    let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
    let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
    c.bench_function(
        "invocations with String new arg and String result",
        move |b| b.iter(|| {
            do_invocation_w_new_args(black_box(&jvm), black_box(&instance))
        }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);