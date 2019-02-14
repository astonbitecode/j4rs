# j4rs

[![crates.io](https://img.shields.io/crates/v/j4rs.svg)](https://crates.io/crates/j4rs)
[![Maven Central](https://img.shields.io/badge/Maven%20Central-0.4.0-blue.svg)](http://search.maven.org/classic/#search%7Cga%7C1%7Cg%3A%22io.github.astonbitecode%22%20AND%20a%3A%22j4rs%22)
![Build Status](https://travis-ci.org/astonbitecode/j4rs.svg?branch=master)
[![Build status](https://ci.appveyor.com/api/projects/status/9k83nufbt958w6p2?svg=true)](https://ci.appveyor.com/project/astonbitecode/j4rs)

j4rs stands for __'Java for Rust'__ and allows effortless calls to Java code, from Rust.

## Usage

### Basics

```rust
use j4rs::{Instance, InvocationArg, Jvm, JvmBuilder};

// Create a JVM
let jvm = JvmBuilder::new().build().unwrap();

// Create a java.lang.String instance
let string_instance = jvm.create_instance(
    "java.lang.String",     // The Java class to create an instance for
    &Vec::new(),            // The `InvocationArg`s to use for the constructor call - empty for this example
).unwrap();

// The instances returned from invocations and instantiations can be viewed as pointers to Java Objects.
// They can be used for further Java calls.
// For example, the following invokes the `isEmpty` method of the created java.lang.String instance
let boolean_instance = jvm.invoke(
    &string_instance,       // The String instance created above
    "isEmpty",              // The method of the String instance to invoke
    &Vec::new(),            // The `InvocationArg`s to use for the invocation - empty for this example
).unwrap();

// If we need to transform an `Instance` to Rust value, the `to_rust` should be called
let rust_boolean: bool = jvm.to_rust(boolean_instance).unwrap();
println!("The isEmpty() method of the java.lang.String instance returned {}", rust_boolean);
// The above prints:
// The isEmpty() method of the java.lang.String instance returned true

// Static invocation
let _static_invocation_result = jvm.invoke_static(
    "java.lang.System",     // The Java class to invoke
    "currentTimeMillis",    // The static method of the Java class to invoke
    &Vec::new(),            // The `InvocationArg`s to use for the invocation - empty for this example
).unwrap();

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
    &Vec::new())
    .unwrap();

// Invoke the method
let instance_receiver_res = jvm.invoke_to_channel(
    &i,                         // The instance to invoke asynchronously
    "performCallback",          // The method to invoke asynchronoysly
    &Vec::new()                 // The `InvocationArg`s to use for the invocation - empty for this example
);

// Wait for the response to come
let instance_receiver = instance_receiver_res.unwrap();
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

### Passing arguments from Rust to Java

j4rs uses the `InvocationArg` enum to pass arguments to the Java world.

Users can benefit of the existing `From` implementations for several basic types:

```rust
let i1 = InvocationArg::from("a str");      // Creates an arg of java.lang.String
let my_string = "a string".to_owned();
let i2 = InvocationArg::from(my_string);    // Creates an arg of java.lang.String
let i3 = InvocationArg::from(true);         // Creates an arg of java.lang.Boolean
let i4 = InvocationArg::from(1_i8);         // Creates an arg of java.lang.Byte
let i5 = InvocationArg::from('c');          // Creates an arg of java.lang.Character
let i6 = InvocationArg::from(1_i16);        // Creates an arg of java.lang.Short
let i7 = InvocationArg::from(1_i64);        // Creates an arg of java.lang.Long
let i8 = InvocationArg::from(0.1_f32);      // Creates an arg of java.lang.Float
let i9 = InvocationArg::from(0.1_f64);      // Creates an arg of java.lang.Double
```

And for `Vec`s:

```rust
let my_vec: Vec<String> = vec![
    "abc".to_owned(),
    "def".to_owned(),
    "ghi".to_owned()];

let i10 = InvocationArg::from((my_vec.as_slice(), &jvm));
```

The `Instance`s returned by j4rs can be transformed to `InvocationArg`s and be used for invoking methods as well:

```rust
let one_more_string_instance = jvm.create_instance(
    "java.lang.String",     // The Java class to create an instance for
    &Vec::new(),            // The `InvocationArg`s to use for the constructor call - empty for this example
).unwrap();

let i11 = InvocationArg::from(one_more_string_instance);
```

### Casting

An `Instance` may be casted to some other Class:

```rust
let instantiation_args = vec![InvocationArg::from("Hi")];
let instance = jvm.create_instance("java.lang.String", instantiation_args.as_ref()).unwrap();
jvm.cast(&instance, "java.lang.Object").unwrap();
```

### Adding jars in the classpath

If we have one jar that needs to be accessed using `j4rs`, we need to add it in the classpath during the JVM creation:

```rust
let entry = ClasspathEntry::new("/home/myuser/dev/myjar-1.0.0.jar");
let jvm: Jvm = JvmBuilder::new()
    .classpath_entry(entry)
    .build()
    .unwrap();
```

## j4rs Java library

The jar for `j4rs` is available in the Maven Central. It may be used by adding the following dependency in a pom:

```xml
<dependency>
    <groupId>io.github.astonbitecode</groupId>
    <artifactId>j4rs</artifactId>
    <version>0.4.0</version>
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
    <version>0.4.0-java7</version>
</dependency>
```

## Prerequisites for the compilation

* [Rust](https://www.rust-lang.org/tools/install) is installed
* Java 8 or higher is installed 
* `$JAVA_HOME` is set

## Prerequisites for running an application that has `j4rs` as dependency

* [Rust](https://www.rust-lang.org/tools/install) is installed.
* Java 8 or higher is installed.
* `$LD_LIBRARY_PATH` is set and includes the directory that contains the `libjvm.so` shared library (`jvm.dll` in Windows).

## Note

The build script attempts to help setting up the environment as needed for the `j4rs` to work. However, things can be different depending on the use case and the environment.

__In a Linux host__, the build script of `j4rs` populates the `$LD_LIBRARY_PATH` and exports it in the `$CARGO_HOME/env` file. This means that it is enough to execute 
```source $CARGO_HOME/env``` in order to export the correct `$LD_LIBRARY_PATH` variable in your session.

## Next?

* Implement macros to facilitate `j4rs` users

 Possibly something like: 

```rust
// Instantiation
let i12 = jnew!(&jvm -> new java.lang.String("a-new-string"));

// Invocation
let i13 = j!(&i12.split("-"));
```

* Improve documentation

## Licence

At your option, under: 

* Apache License, Version 2.0, (http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (http://opensource.org/licenses/MIT)
