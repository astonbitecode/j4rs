/*
 * Copyright 2018 astonbitecode
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package org.astonbitecode.j4rs.api.dtos;

import org.astonbitecode.j4rs.api.NativeInvocation;
import org.astonbitecode.j4rs.errors.InvalidArgumentException;
import org.astonbitecode.j4rs.utils.Defs;

public class InvocationArg {
    /**
     * The array contents should map to a List. This is in order to allow calls of type Arrays.asList(arg1, arg2, arg3, ...)
     */
    public static final String CONTENTS_ARRAY = "org.astonbitecode.j4rs.api.dtos.Array";
    private NativeInvocation nativeInvocation;
    private String json;
    /**
     * If java, the argument is taken straight by the Java code as Object. If
     * rust, the argument is a json document that need to be deserialized to an
     * Object.
     */
    private String argFrom;
    /**
     * The type of this argument. This is used when json objects come from Rust, in order to be mapped to proper Java Objects.
     */
    private String className;
    private static String JAVA = Defs.JAVA;
    private static String RUST = Defs.RUST;

    public InvocationArg(String className, NativeInvocation nativeInvocation) {
        this.className = className;
        this.nativeInvocation = nativeInvocation;
        this.argFrom = JAVA;
    }

    public InvocationArg(NativeInvocation nativeInvocation) {
        this.className = nativeInvocation.getClass().getName();
        this.nativeInvocation = nativeInvocation;
        this.argFrom = JAVA;
    }

    public InvocationArg(String className, String json) {
        this.className = className;
        this.json = json;
        this.argFrom = RUST;
    }

    /**
     * If java, the argument is taken straight by the Java code as Object. If
     * rust, the argument is a json document that need to be deserialized to an
     * Object.
     *
     * @return The The argFrom
     */
    public String getArgFrom() {
        return argFrom;
    }

    /**
     * The type of this argument. This is used when json objects come from Rust, in order to be mapped to proper Java Objects.
     *
     * @return The classname
     */
    public String getClassName() {
        return className;
    }

    public NativeInvocation getNativeInvocation() {
        if (argFrom.equals(RUST)) {
            throw new InvalidArgumentException("This InvocationArg of class " + className + " is created by Rust code.");
        }
        return nativeInvocation;
    }

    public String getJson() {
        if (argFrom.equals(JAVA)) {
            throw new InvalidArgumentException("This InvocationArg of class " + className + " is created by Java code.");
        }
        return json;
    }

    @Override
    public String toString() {
        return "classname:" + this.className + ", argFrom:" + this.argFrom + ", json:" + this.json + ", nativeInvocation:" + this.nativeInvocation;
    }
}
