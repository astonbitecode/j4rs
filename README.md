# j4rs

[![crates.io](https://img.shields.io/crates/v/j4rs.svg)](https://crates.io/crates/j4rs)
[![Maven Central](https://img.shields.io/badge/Maven%20Central-0.17.0-blue.svg)](http://search.maven.org/classic/#search%7Cga%7C1%7Cg%3A%22io.github.astonbitecode%22%20AND%20a%3A%22j4rs%22)
![Build](https://github.com/astonbitecode/j4rs/actions/workflows/ci-workflow.yml/badge.svg)

j4rs stands for __'Java for Rust'__ and allows effortless calls to Java code from Rust and vice-versa.

## Features

* **Rust to Java direction support (call Java from Rust).**
    * No special configuration needed (no need to tweak LD_LIBRARY_PATH, PATH etc).
    * [Easily instantiate and invoke Java classes.](#Basics)
    * [Support custom types via serialization.](#Passing-custom-arguments-from-Rust-to-Java)
    * [.async/.await support](#Async-support)
    * [Casting support.](#Casting)
    * [Java arrays / variadic support.](#Java-arrays-and-variadics)
    * [Java generics support.](#Java-Generics)
    * [Java primitives support.](#Java-primitives)
    * [Java instances invocations chaining.](#Java-instances-chaining)
    * [Java -> Rust callbacks support.](#Callback-support)
    * [Simple Maven artifacts download and deployment.](#Using-Maven-artifacts)
* **[Java -> Rust support](#Java-to-Rust-support) (Call Rust from Java).**
* **[JavaFX support](#JavaFX-support) (including FXML support).**
* **Tested on Linux, Windows and Android.**

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

// Access a field of a class
let system_class = jvm.static_class("java.lang.System")?;
let system_out_field = jvm.field(&system_class, "out");

// Retrieve an enum constant using the field
let access_mode_enum = jvm.static_class("java.nio.file.AccessMode")?;
let access_mode_write = jvm.field(&access_mode_enum, "WRITE")?;
```

`Instances`s of Java `List`s and `Map`s can be created with the `java_list` and `java_map` functions:

```rust
let rust_vec = vec!["arg1", "arg2", "arg3", "arg33"];

// Generate a Java List. The Java List implementation is the one that is returned by java.util.Arrays#asList
let java_list_instance = jvm.java_list(
    JavaClass::String,
    rust_vec)?;

let rust_map = HashMap::from([
    ("Potatoes", 3),
    ("Tomatoes", 33),
    ("Carrotoes", 333),
]);

// Generate a java.util.HashMap.
let java_map_instance = jvm.java_map(
    JavaClass::String,
    JavaClass::Integer,
    map)?;
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

### Passing custom arguments from Rust to Java

Custom types, for which there is no `TryFrom` implementation, are also supported via serialization.

To use a custom struct `MyBean` as an `InvocationArg` it needs to be serializable:

```rust
#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct MyBean {
    someString: String,
    someInteger: isize,
}
```

Then, an `InvocationArg` can be created like:

```rust
let my_bean = MyBean {
    someString: "My String In A Bean".to_string(),
    someInteger: 33,
};
let ia = InvocationArg::new(&my_bean, "org.astonbitecode.j4rs.tests.MyBean");
```

And it can be used as an argument to a Java method that accepts `org.astonbitecode.j4rs.tests.MyBean` instances.

Of course, there should exist a respective Java class in the classpath for the deserialization to work and the custom Java Object to be created:

```java
package org.astonbitecode.j4rs.tests;

public class MyBean {
    private String someString;
    private Integer someInteger;

    public MyBean() {
    }

    public String getSomeString() {
        return someString;
    }

    public void setSomeString(String someString) {
        this.someString = someString;
    }

    public Integer getSomeInteger() {
        return someInteger;
    }

    public void setSomeInteger(Integer someInteger) {
        this.someInteger = someInteger;
    }
}
```

### Async support
(v0.16.0 onwards)

`j4rs` supports `.async/.await` via`Jvm::invoke_async` function.
The function returns a [Future](https://docs.rs/futures/latest/futures/future/trait.Future.html), which is completed via the `Receiver` of a [oneshot channel](https://docs.rs/futures/latest/futures/channel/oneshot/fn.channel.html). 

In Java side, the methods that can be invoked by `invoke_async`, __must__ return a Java [Future](https://docs.oracle.com/en/java/javase/17/docs/api/java.base/java/util/concurrent/Future.html). 
When the Java Future completes, the Java side of `j4rs` invokes native Rust code that completes the pending Rust `Future` 
with either success or failure, using the `Sender` of the oneshot channel that was created when the `invoke_async` was called.

For example, assuming we have a Java method that returns a Future:

```java
package org.astonbitecode.j4rs.tests;

public class MyTest {
  private static ExecutorService executor = Executors.newSingleThreadExecutor();

  // Just return the passed String in a Future
  public Future<String> getStringWithFuture(String string) {
    CompletableFuture<String> completableFuture = new CompletableFuture<>();
    executor.submit(() -> {
      completableFuture.complete(string);
      return null;
    });
    return completableFuture;
  }
}
```

We can invoke it like following:

```rust
let s_test = "j4rs_rust";
let my_test = jvm.create_instance("org.astonbitecode.j4rs.tests.MyTest", &[])?;
let instance = jvm.invoke_async(&my_test, "getStringWithFuture", &[InvocationArg::try_from(s_test)?]).await?;
let string: String = jvm.to_rust(instance)?;
assert_eq!(s_test, string);
```

Please note that it is better for the Java methods that are invoked by the `invoke_async` function 
to return a [CompletableFuture](https://docs.oracle.com/en/java/javase/17/docs/api/java.base/java/util/concurrent/CompletableFuture.html), 
as this improves performance.

`j4rs` handles simple Java Futures that are not `CompletableFuture`s with [polling](https://github.com/astonbitecode/j4rs/blob/86a2cb7bee10e5941fd0cada00afc355ea9e3ebb/java/src/main/java/org/astonbitecode/j4rs/api/async/J4rsPolledFuture.java#L25), 
using an internal one-threaded `ScheduledExecutorService`. 

This has apparent performance issues.

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
    <version>0.15.3</version>
    <scope>provided</scope>
</dependency>
```

Note that the `scope` is `provided`. This is because the `j4rs` Java resources are always available with the `j4rs` crate.

Use like this in order to avoid possible classloading errors.

## j4rs in android

### Rust side

1. Define your crate as cdylib in the `Cargo.toml`:

```toml
[lib]
name = "myandroidapp"
crate-type = ["cdylib"]
```

2. Implement a `jni_onload` function and apply the provided `JavaVM`
   to the `j4rs` like following:

```rust
const JNI_VERSION_1_6: jint = 0x00010006;

#[allow(non_snake_case)]
#[no_mangle]
pub extern fn jni_onload(env: *mut JavaVM, _reserved: jobject) -> jint {
    j4rs::set_java_vm(env);
    jni_version_1_6
}
```

### Java side

Create an `Activity` and define your native methods normally, as described [here](#java-to-rust-support).

Note:
If you encounter any issues when using j4rs in Android, this may be caused by Java 8 compatibility problems. This is why there is a `Java 7` version of `j4rs`:

```xml
<dependency>
    <groupId>io.github.astonbitecode</groupId>
    <artifactId>j4rs</artifactId>
    <version>0.13.1-java7</version>
</dependency>
```

## JavaFX support
(v0.13.0 onwards)

### Steps to build a JavaFX UI

#### 1. Have Rust, cargo and JDK 11 (or above) installed

#### 2. Retrieve the JavaFX dependencies for j4rs:

A good idea is that this happens during build time, in order the dependencies to be available when the actual Rust application starts and the JVM is initialized.
This can happen by adding the following in a [build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html?highlight=build,scrpit#build-scripts):

```rust
	use j4rs::JvmBuilder;
use j4rs::jfx::JavaFxSupport;

fn main() {
    let jvm = JvmBuilder::new().build().unwrap();
    jvm.deploy_javafx_dependencies().unwrap();
}

```

#### 3. Implement the UI:

There are two choices here; either build the UI using FXML, or, build it traditionally, using Java code.
In the code snippets below, you may find comments with a short description for each line.

##### 3.a Implement the UI with Java calls to the JavaFX API

```rust
// Create a Jvm with JavaFX support
let jvm = JvmBuilder::new().with_javafx_support().build()?;

// Start the JavaFX application.
// When the JavaFX application starts, the `InstanceReceiver` channel that is returned from the `start_javafx_app` invocation
// will receive an Instance of `javafx.stage.Stage`.
// The UI may start being built using the provided `Stage`.
let stage = jvm.start_javafx_app()?.rx().recv()?;

// Create a StackPane. Java code: StackPane root = new StackPane();
let root = jvm.create_instance("javafx.scene.layout.StackPane", &[])?;

// Create the button. Java code: Button btn = new Button();
let btn = jvm.create_instance("javafx.scene.control.Button", &[])?;
// Get the action channel for this button
let btn_action_channel = jvm.get_javafx_event_receiver(&btn, FxEventType::ActionEvent_Action)?;
// Set the text of the button. Java code: btn.setText("Say Hello World to Rust");
jvm.invoke(&btn, "setText", &["A button that sends events to Rust".try_into()?])?;
// Add the button to the GUI. Java code: root.getChildren().add(btn);
jvm.chain(&root)?
.invoke("getChildren", &[])?
.invoke("add", &[btn.try_into()?])?
.collect();

// Create a new Scene. Java code: Scene scene = new Scene(root, 300, 250);
let scene = jvm.create_instance("javafx.scene.Scene", &[
root.try_into()?,
InvocationArg::try_from(300_f64)?.into_primitive()?,
InvocationArg::try_from(250_f64)?.into_primitive()?])?;
// Set the title for the scene. Java code: stage.setTitle("Hello Rust world!");
jvm.invoke(&stage, "setTitle", &["Hello Rust world!".try_into()?])?;
// Set the scene in the stage. Java code: stage.setScene(scene);
jvm.invoke(&stage, "setScene", &[scene.try_into()?])?;
// Show the stage. Java code: stage.show();
jvm.invoke(&stage, "show", &[])?;

```

##### 3.b Implement the UI with [FXML](https://openjfx.io/javadoc/12/javafx.fxml/javafx/fxml/doc-files/introduction_to_fxml.html#overview)

I personally prefer building the UI with FXMLs, using for example the [Scene Builder](https://gluonhq.com/products/scene-builder/).

The thing to keep in mind is that the controller class should be defined in the root FXML element and it should be `fx:controller="org.astonbitecode.j4rs.api.jfx.controllers.FxController"`

Here is an FXML example; it creates a window with a label and a button:

```xml
<?xml version="1.0" encoding="UTF-8"?>

<?import javafx.scene.control.Button?>
<?import javafx.scene.control.Label?>
<?import javafx.scene.layout.HBox?>
<?import javafx.scene.layout.VBox?>
<?import javafx.scene.text.Font?>

<VBox alignment="TOP_CENTER" maxHeight="-Infinity" maxWidth="-Infinity" minHeight="-Infinity" minWidth="-Infinity" prefHeight="400.0" prefWidth="725.0" spacing="33.0" xmlns="http://javafx.com/javafx/11.0.1" xmlns:fx="http://javafx.com/fxml/1" fx:controller="org.astonbitecode.j4rs.api.jfx.controllers.FxController">
    <children>
        <Label text="JavaFX in Rust">
            <font>
                <Font size="65.0" />
            </font>
        </Label>
        <Label text="This UI is loaded with a FXML file" />
        <HBox alignment="CENTER" prefHeight="100.0" prefWidth="200.0" spacing="10.0">
            <children>
                <Button id="helloButton" mnemonicParsing="false" text="Say Hello" />
            </children>
        </HBox>
    </children>
</VBox>

```

The `id` of the elements can be used to retrieve the respective [Nodes](https://openjfx.io/javadoc/13/javafx.graphics/javafx/scene/Node.html) in Rust and act upon them (eg. adding Event Listeners, changing the texts or effects on them etc).

```rust
// Create a Jvm with JavaFX support
let jvm = JvmBuilder::new().with_javafx_support().build()?;

// Start the JavaFX application.
// When the JavaFX application starts, the `InstanceReceiver` channel that is returned from the `start_javafx_app` invocation
// will receive an Instance of `javafx.stage.Stage`.
// The UI may start being built using the provided `Stage`.
let stage = jvm.start_javafx_app()?.rx().recv()?;

// Set the title for the scene. Java code: stage.setTitle("Hello Rust world!");
jvm.invoke(&stage, "setTitle", &["Hello JavaFX from Rust!".try_into()?])?;
// Show the stage. Java code: stage.show();
jvm.invoke(&stage, "show", &[])?;

// Load a fxml. This returns an `FxController` which can be used in order to find Nodes by their id,
// add Event Listeners and more.
let controller = jvm.load_fxml(&PathBuf::from("./fxml/jfx_in_rust.fxml"), &stage)?;

// Wait for the controller to be initialized. This is not mandatory, it is here to shoe that the functionality exists.
let _ = controller.on_initialized_callback(&jvm)?.rx().recv()?;
println!("The controller is initialized!");

// Get the InstanceReceiver to retrieve callbacks from the JavaFX button with id helloButton
let hello_button_action_channel = controller.get_event_receiver_for_node("helloButton", FxEventType::ActionEvent_Action, &jvm)?;

```

For a complete example, please have a look [here](https://github.com/astonbitecode/j4rs-showcase).

## Java to Rust support

(v0.12.0 onwards)

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

## Portability assumptions after Rust build (shipping a j4rs application)

During build, `j4rs` creates a `jassets` directory which contains the "java world" that is needed for the crate to work.
It is always automatically populated with Java libraries and can be considered something like a default classpath container that should always be available.

By default, `jassets` lies in the same directory with the crate-generated artifacts (under [CARGO_TARGET_DIR](https://doc.rust-lang.org/cargo/reference/environment-variables.html)), so there should not be any issues during development.

But how can the application be shipped after the implementation is done?

Someone may specify a different [base_path](https://docs.rs/j4rs/0.13.0/j4rs/struct.JvmBuilder.html#method.with_base_path) for j4rs during the Jvm initialization, issuing something like:

```rust
let jvm_res = j4rs::JvmBuilder::new()
.with_base_path("/opt/myapp")
.build();
```

The `base_path` defines the location of two directories that are needed for j4rs to work;
namely `jassets` and `deps`.

1. __jassets__ contains the j4rs jar and other jars that may be deployed using [Maven](https://github.com/astonbitecode/j4rs#Using-Maven-artifacts).
2. __deps__ should contain the j4rs dynamic library. This is needed to achieve  [callbacks](https://github.com/astonbitecode/j4rs#Callback-support) from java to rust.
   The `deps` dir is not needed if the application does not execute Java->Rust callbacks.

So, someone may have their application binary under eg. `/usr/bin`, and the `jassets` and `deps` directories under `/opt/myapp/`, or `$HOME/.myapp`, or anywhere else.

An example directory tree could be:

```
/ 
+ --- usr
|      + --- bin
|             + --- myapp
| 
+ --- opt
       + --- myapp 
              + --- jassets
              + --- deps
```

Moreover, there is also a [utility function](https://docs.rs/j4rs/0.13.0/j4rs/struct.Jvm.html#method.copy_j4rs_libs_under) that automatically performs copying of the two directories under a specific path.
The `Jvm::copy_j4rs_libs_under` function can be called by the build script of the crate that is being shipped:

```rust
Jvm::copy_j4rs_libs_under("/opt/myapp")?;
```

After that, `/opt/myapp` will contain everything that is needed in order `j4rs` to work,
as long as the Jvm creation is done using the `with_base_path` method:

```rust
let jvm_res = j4rs::JvmBuilder::new()
.with_base_path("/opt/myapp")
.build();
```

## FAQ

### I get `java.lang.NoSuchMethodError: java.net.URLClassLoader.<init>(Ljava/lang/String;[Ljava/net/URL;Ljava/lang/ClassLoader;)V`

`j4rs` uses a custom ClassLoader, that needs minimum Java version 9. In order to use the default classloader that supports 
older Java versions, invoke the `JvmBuilder::with_default_classloader` when building the `Jvm`.

### How can I enable debug logging?

`j4rs` uses the [log crate](https://docs.rs/log/latest/log/), so, logging may be configured accordingly, depending on the chosen implementation.

However, it also supports console logging, which is configured with setting the env var `J4RS_CONSOLE_LOG_LEVEL`.

Accepted values are `debug`, `info`, `warn`, `error` and `disabled`.

## Licence

At your option, under:

* Apache License, Version 2.0, (http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (http://opensource.org/licenses/MIT)
