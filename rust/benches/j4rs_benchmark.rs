#[macro_use]
extern crate criterion;

use std::convert::TryFrom;

use criterion::black_box;
use criterion::Criterion;

use j4rs::{self, Instance, InvocationArg, Jvm};

fn do_instance_creation(jvm: &Jvm) -> Instance {
    jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap()
}

fn do_invocation_w_no_args(jvm: &Jvm, instance: &Instance) -> Instance {
    jvm.invoke(instance, "getMyString", &[]).unwrap()
}

fn do_invocation_w_string_args(jvm: &Jvm, instance: &Instance) -> Instance {
    jvm.invoke(instance, "echo", &vec![InvocationArg::try_from("a").unwrap()]).unwrap()
}

fn do_invocation_w_integer_args(jvm: &Jvm, instance: &Instance) -> Instance {
    jvm.invoke(instance, "echo", &vec![InvocationArg::try_from(33_i32).unwrap()]).unwrap()
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
            do_invocation_w_string_args(black_box(&jvm), black_box(&instance))
        }));

    let jvm: Jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();
    let instance = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[]).unwrap();
    c.bench_function(
        "invocations with Integer arg and Integer result",
        move |b| b.iter(|| {
            do_invocation_w_integer_args(black_box(&jvm), black_box(&instance))
        }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);