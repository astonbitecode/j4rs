# j4rs

[![crates.io](https://img.shields.io/crates/v/j4rs.svg)](https://crates.io/crates/j4rs)
[![Maven Central](https://img.shields.io/badge/Maven%20Central-0.12.0-blue.svg)](http://search.maven.org/classic/#search%7Cga%7C1%7Cg%3A%22io.github.astonbitecode%22%20AND%20a%3A%22j4rs%22)
[![Build Status](https://travis-ci.org/astonbitecode/j4rs.svg?branch=master)](https://travis-ci.org/astonbitecode/j4rs/)
[![Build status](https://ci.appveyor.com/api/projects/status/9k83nufbt958w6p2?svg=true)](https://ci.appveyor.com/project/astonbitecode/j4rs)

j4rs stands for __'Java for Rust'__ and allows effortless calls to Java code from Rust and vice-versa.

# Info

`j4rs` focused solely in facilitating Rust applications in making calls to Java code 
by [allowing]((#Basics)) JVM creation and manipulation from the Rust code, efortless Java method invocations,
Java libraries [provisioning via Maven](#Using-Maven-artifacts) and [more](#Features).

This has changed since release 0.12.0. 
***j4rs can now be used as well in Java projects that want to achieve JNI calls to Rust libraries.***

## Features

* No special configuration needed (no need to tweak LD_LIBRARY_PATH, PATH etc).
* [Easily instantiate and invoke Java classes.](#Basics)
* [Casting support.](#Casting)
* [Java arrays / variadic support.](#Java-arrays-and-variadics)
* [Java generics support.](#Java-Generics)
* [Java primitives support.](#Java-primitives)
* [Java instances invocations chaining.](#Java-instances-chaining)
* [Java -> Rust callbacks support.](#Callback-support)
* [Simple Maven artifacts download and deployment.](#Using-Maven-artifacts)
* Tested on Linux, Windows and Android.
* [Java -> Rust support](#Java-to-Rust-support).

## Usage

### Basics

```rust
use j4rs::{Instance, InvocationArg, Jvm, JvmBuilder};

// Create a JVM
let jvm = JvmBuilder::new().build()?;

// Create a java.lang.String instance
let string_instance = jvm.create_instance(
    "java.lang.String",     // The Java class to create an instance for
    &Vec::new(),            // The `InvocationArg`s to use for the constructor call - empty for this example
)?;

// The instances returned from invocations and instantiations can be viewed as pointers to Java Objects.
// They can be used for further Java calls.
// For example, the following invokes the `isEmpty` method of the created java.lang.String instance
let boolean_instance = jvm.invoke(
    &string_instance,       // The String instance created above
    "isEmpty",              // The method of the String instance to invoke
    &Vec::new(),            // The `InvocationArg`s to use for the invocation - empty for this example
)?;

// If we need to transform an `Instance` to Rust value, the `to_rust` should be called
let rust_boolean: bool = jvm.to_rust(boolean_instance)?;
println!("The isEmpty() method of the java.lang.String instance returned {}", rust_boolean);
// The above prints:
// The isEmpty() method of the java.lang.String instance returned true

// Static invocation
let _static_invocation_result = jvm.invoke_static(
    "java.lang.System",     // The Java class to invoke
    "currentTimeMillis",    // The static method of the Java class to invoke
    &Vec::new(),            // The `InvocationArg`s to use for the invocation - empty for this example
)?;

```

### Passing arguments from Rust to Java

j4rs uses the `InvocationArg` enum to pass arguments to the Java world.

Users can benefit of the existing `TryFrom` implementations for several basic types:

```rust
let i1 = InvocationArg::try_from("a str")?;      // Creates an arg of java.lang.String
let my_string = "a string".to_owned();
let i2 = InvocationArg::try_from(my_string)?;    // Creates an arg of java.lang.String
let i3 = InvocationArg::try_from(true)?;         // Creates an arg of java.lang.Boolean
let i4 = InvocationArg::try_from(1_i8)?;         // Creates an arg of java.lang.Byte
let i5 = InvocationArg::try_from('c')?;          // Creates an arg of java.lang.Character
let i6 = InvocationArg::try_from(1_i16)?;        // Creates an arg of java.lang.Short
let i7 = InvocationArg::try_from(1_i64)?;        // Creates an arg of java.lang.Long
let i8 = InvocationArg::try_from(0.1_f32)?;      // Creates an arg of java.lang.Float
let i9 = InvocationArg::try_from(0.1_f64)?;      // Creates an arg of java.lang.Double
```

And for `Vec`s:

```rust
let my_vec: Vec<String> = vec![
    "abc".to_owned(),
    "def".to_owned(),
    "ghi".to_owned()];

let i10 = InvocationArg::try_from(my_vec.as_slice())?;
```

The `Instance`s returned by j4rs can be transformed to `InvocationArg`s and be further used for invoking methods as well:

```rust
let one_more_string_instance = jvm.create_instance(
    "java.lang.String",     // The Java class to create an instance for
    &Vec::new(),            // The `InvocationArg`s to use for the constructor call - empty for this example
)?;

let i11 = InvocationArg::try_from(one_more_string_instance)?;
```

To create an `InvocationArg` that represents a `null` Java value, use the `From` implementation with the `Null` struct:

```rust
let null_string = InvocationArg::from(Null::String);                // A null String
let null_integer = InvocationArg::from(Null::Integer);              // A null Integer
let null_obj = InvocationArg::from(Null::Of("java.util.List"));    // A null object of any other class. E.g. List
```

### Casting

An `Instance` may be casted to some other Class:

```rust
let instantiation_args = vec![InvocationArg::try_from("Hi")?];
let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref())?;
jvm.cast(&instance, "java.lang.Object")?;
```

### Java arrays and variadics

```rust
// Create a Java array of Strings
let s1 = InvocationArg::try_from("string1")?;
let s2 = InvocationArg::try_from("string2")?;
let s3 = InvocationArg::try_from("string3")?;

let arr_instance = jvm.create_java_array("java.lang.String", &vec![s1, s2, s3])?;
// Invoke the Arrays.asList(...) and retrieve a java.util.List<String>
let list_instance = jvm.invoke_static("java.util.Arrays", "asList", &[InvocationArg::from(arr_instance)])?;
```

### Java Generics

```rust
// Assuming that the following map_instance is a Map<String, Integer>
// we may invoke its put method
jvm.invoke(&map_instance, "put", &vec![InvocationArg::try_from("one")?, InvocationArg::try_from(1)?])?;
```

### Java primitives

Even if auto boxing and unboxing is in place, `j4rs` cannot invoke methods with _primitive_ int arguments using _Integer_ instances.

For example, the following code does not work:

```rust
let ia = InvocationArg::try_from(1_i32)?;
jvm.create_instance("java.lang.Integer", &[ia])?;
``` 

It throws an _InstantiationException_ because the constructor of `Integer` takes a primitive `int` as an argument:

>Exception in thread "main" org.astonbitecode.j4rs.errors.InstantiationException: Cannot create instance of java.lang.Integer
  	at org.astonbitecode.j4rs.api.instantiation.NativeInstantiationImpl.instantiate(NativeInstantiationImpl.java:37)
  Caused by: java.lang.NoSuchMethodException: java.lang.Integer.<init>(java.lang.Integer)
  	at java.base/java.lang.Class.getConstructor0(Class.java:3349)
  	at java.base/java.lang.Class.getConstructor(Class.java:2151)
  	at org.astonbitecode.j4rs.api.instantiation.NativeInstantiationImpl.createInstance(NativeInstantiationImpl.java:69)
  	at org.astonbitecode.j4rs.api.instantiation.NativeInstantiationImpl.instantiate(NativeInstantiationImpl.java:34)

In situations like this, the `java.lang.Integer` instance should be transformed to a primitive `int` first:

```rust
let ia = InvocationArg::try_from(1_i32)?.into_primitive()?;
jvm.create_instance("java.lang.Integer", &[ia]);
```

### Java instances chaining
```rust
use j4rs::{Instance, InvocationArg, Jvm, JvmBuilder};

// Create a JVM
let jvm = JvmBuilder::new().build()?;

// Create an instance
let string_instance = jvm.create_instance(
    "java.lang.String",
    &vec![InvocationArg::try_from(" a string ")?],
)?;

// Perform chained operations on the instance
let string_size: isize = jvm.chain(string_instance)
    .invoke("trim", &[])?
    .invoke("length", &[])?
    .to_rust()?;

// Assert that the string was trimmed
assert!(string_size == 8);
```

### Callback support

`j4rs` provides support for _Java to Rust callbacks_.

These callbacks come to the Rust world via Rust [Channels](https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html). 

In order to initialize a channel that will provide Java callback values, the `Jvm::invoke_to_channel` should be called. It returns a result of `InstanceReceiver` struct, which contains a Channel [Receiver](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html):

```rust
// Invoke of a method of a Java instance and get the returned value in a Rust Channel.

// Create an Instance of a class that supports Native Callbacks
// (the class just needs to extend the 
// `org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport`)
let i = jvm.create_instance(
    "org.astonbitecode.j4rs.tests.MyTest",
    &Vec::new())?;

// Invoke the method
let instance_receiver_res = jvm.invoke_to_channel(
    &i,                         // The instance to invoke asynchronously
    "performCallback",          // The method to invoke asynchronoysly
    &Vec::new()                 // The `InvocationArg`s to use for the invocation - empty for this example
);

// Wait for the response to come
let instance_receiver = instance_receiver_res?;
let _ = instance_receiver.rx().recv();
```

In the Java world, a Class that can do __Native Callbacks__ must extend the 
`org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport`

For example, consider the following Java class. 

The `performCallback` method spawns a new Thread and invokes the `doCallback` method in this Thread. The `doCallback` method is inherited by the `NativeCallbackToRustChannelSupport` class.

```java
package org.astonbitecode.j4rs.tests;

import org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport;

public class MyTest extends NativeCallbackToRustChannelSupport {

    public void performCallback() {
        new Thread(() -> {
            doCallback("THIS IS FROM CALLBACK!");
        }).start();
    }

}
```

### Using Maven artifacts

Since release 0.6.0 there is the possibility to download Java artifacts from the Maven repositories.
While it is possible to define more repos, the [maven central](https://search.maven.org/) is by default and always available.

For example, here is how the dropbox dependency can be downloaded and get deployed to be used by the rust code:

```rust
let dbx_artifact = MavenArtifact::from("com.dropbox.core:dropbox-core-sdk:3.0.11");
jvm.deploy_artifact(dbx_artifact)?;
```

Additional artifactories can be used as well:

```rust
let jvm: Jvm = JvmBuilder::new()
    .with_maven_settings(MavenSettings::new(vec![
        MavenArtifactRepo::from("myrepo1::https://my.repo.io/artifacts"),
        MavenArtifactRepo::from("myrepo2::https://my.other.repo.io/artifacts")])
    )
    .build()
    ?;

jvm.deploy_artifact(&MavenArtifact::from("io.my:library:1.2.3"))?;
```

Maven artifacts are added automatically to the classpath and do not need to be explicitly added.

A good practice is that the deployment of maven artifacts is done by build scripts, during the crate's compilation. This ensures that the classpath is properly populated during the actual Rust code execution.

_Note: the deployment does not take care the transitive dependencies yet._  

### Adding jars to the classpath

If we have one jar that needs to be accessed using `j4rs`, we need to add it in the classpath during the JVM creation:

```rust
let entry = ClasspathEntry::new("/home/myuser/dev/myjar-1.0.0.jar");
let jvm: Jvm = JvmBuilder::new()
    .classpath_entry(entry)
    .build()?;
```

## j4rs Java library

The jar for `j4rs` is available in the Maven Central. It may be used by adding the following dependency in a pom:

```xml
<dependency>
    <groupId>io.github.astonbitecode</groupId>
    <artifactId>j4rs</artifactId>
    <version>0.12.0</version>
    <scope>provided</scope>
</dependency>
```

Note that the `scope` is `provided`. This is because the `j4rs` Java resources are always available with the `j4rs` crate. 

Use like this in order to avoid possible classloading errors.

## j4rs Java library in Android

If you encounter any issues when using j4rs in Android, this may be caused by Java 8 compatibility problems. This is why there is a `Java 7` version of `j4rs`:

```xml
<dependency>
    <groupId>io.github.astonbitecode</groupId>
    <artifactId>j4rs</artifactId>
    <version>0.12.0-java7</version>
</dependency>
```

## Java to Rust support

* Add the two needed dependencies (`j4rs` and `j4rs_derive`) in the `Cargo.toml` 
and mark the project as a `cdylib`, in order to have a shared library as output. 
This library will be loaded and used by the Java code to achieve JNI calls.

* Annotate the functions that will be accessible from the Java code with the `call_from_java` attribute:

```rust
#[call_from_java("io.github.astonbitecode.j4rs.example.RustSimpleFunctionCall.fnnoargs")]
fn my_function_with_no_args() {
    println!("Hello from the Rust world!");
    // If you need to have a Jvm here, you need to attach the thread
    let jvm = Jvm::attach_thread().unwrap();
    // Now you may further call Java classes and methods as usual!
}
```

For a complete example, please have a look [here](https://github.com/astonbitecode/j4rs-java-call-rust).

*Note: JNI is used behind the scenes, so, any [conventions in naming](https://docs.oracle.com/javase/7/docs/technotes/guides/jni/spec/design.html#wp133) that hold for JNI, should hold for `j4rs` too. 
For example, underscores (`_`) should be escaped and become `_1` in the `call_from_java` definition.*

## Licence

At your option, under: 

* Apache License, Version 2.0, (http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (http://opensource.org/licenses/MIT)
