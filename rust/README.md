# j4rs

[![crates.io](https://img.shields.io/crates/v/j4rs.svg)](https://crates.io/crates/j4rs)
[![Maven Central](https://img.shields.io/badge/Maven%20Central-0.1.5-blue.svg)](http://search.maven.org/classic/#search%7Cga%7C1%7Cg%3A%22io.github.astonbitecode%22%20AND%20a%3A%22j4rs%22)
![Build Status](https://travis-ci.org/astonbitecode/j4rs.svg?branch=master)

j4rs stands for __'Java for Rust'__ and allows effortless calls to Java code, from Rust.

## Usage

### Basics

```rust
use j4rs::{Instance, InvocationArg, Jvm};

// Create a JVM
let jvm = j4rs::new_jvm(Vec::new(), Vec::new()).unwrap();

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

`j4rs` provides the means for _Java to Rust callbacks_. 

In the Java world, a Class that can do __Native Callbacks__ must extend the class 
`org.astonbitecode.j4rs.api.invocation.NativeCallbackSupport`

For example, consider the following Java class. 

The `performCallback` method spawns a new Thread and invokes the `doCallback` method in this Thread. The `doCallback` method is inherited by the `NativeCallbackSupport` class.

```java
package org.astonbitecode.j4rs.tests;

import org.astonbitecode.j4rs.api.invocation.NativeCallbackSupport;

public class MyTest extends NativeCallbackSupport {

    public void performCallback() {
        new Thread(() -> {
            doCallback("THIS IS FROM CALLBACK!");
        }).start();
    }

}
```

In the Rust world, the _asynchronous_ invocation result will come from Java via a function that should be defined. This function should be of type `fn(Jvm, Instance) -> ()`

```rust
fn my_callback(jvm: Jvm, inst: Instance) {
    let string_from_java: String = jvm.to_rust(inst).unwrap();
    println!("Asynchronously got from Java: {}", string_from_java);
}
```

We can asynchronously invoke the `performCallback` method by calling the `invoke_async` of the `Jvm`.

```rust
// Asynchronous invocation of a method of a Java instance. The invocation result will come from Java via a callback
// Create an Instance of a class that supports Native Callbacks
// (the class just needs to extend the `org.astonbitecode.j4rs.api.invocation.NativeCallbackSupport`)
let i = jvm.create_instance(
    "org.astonbitecode.j4rs.tests.MyTest",
    &Vec::new())
    .unwrap();

// Invoke asynchronously the method
let _ = jvm.invoke_async(
    &i,                         // The instance to invoke asynchronously
    "performCallback",          // The method to invoke asynchronoysly
    &Vec::new(),                // The `InvocationArg`s to use for the invocation - empty for this example
    my_callback,                // A function of type `fn(Jvm, Instance) -> ()`
);

// Wait a little bit in order to see the callback
// We should see the following in the console:
// Asynchronously got from Java: THIS IS FROM CALLBACK!
let ten_millis = time::Duration::from_millis(1000);
thread::sleep(ten_millis);
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
let classpath_entry = ClasspathEntry::new("/home/myuser/dev/myjar-1.0.0.jar");
let jvm = j4rs::new_jvm(vec![classpath_entry], Vec::new()).unwrap();
```

## j4rs Java library

The jar for `j4rs` is available in the Maven Central. It may be used by adding the following dependency in a pom:

```xml
<dependency>
    <groupId>io.github.astonbitecode</groupId>
    <artifactId>j4rs</artifactId>
    <version>0.1.5</version>
    <scope>provided</scope>
</dependency>
```

Note that the `scope` is `provided`. This is because the `j4rs` Java resources are always available with the `j4rs` crate. 

Use like this in order to avoid possible classloading errors.

## Next?

* Implement macros to facilitate `j4rs` users

 Possibly something like: 

```rust
// Instantiation
let i12 = jnew!(&jvm -> new java.lang.String("a-new-string"));

// Invocation
let i13 = j!(&i12.split("-"));
```

* Fix sharing and using the created `Jvm`s in different Rust threads.

## Licence

At your option, under: 

* Apache License, Version 2.0, (http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (http://opensource.org/licenses/MIT)
