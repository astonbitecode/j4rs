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

import org.astonbitecode.j4rs.api.Instance;
import org.astonbitecode.j4rs.api.invocation.JsonInvocationImpl;
import org.astonbitecode.j4rs.errors.InvalidArgumentException;
import org.astonbitecode.j4rs.utils.Utils;

public class InvocationArg implements Instance {
    /**
     * The array contents should map to a List. This is in order to allow calls of type Arrays.asList(arg1, arg2, arg3, ...)
     */
    public static final String CONTENTS_ARRAY = "org.astonbitecode.j4rs.api.dtos.Array";
    private final Instance instance;
    private final String json;
    /**
     * If not serialized, the argument is taken straight by the Java code as Object.
     * Otherwise, the argument is a json document that needs to be deserialized to an
     * Object.
     */
    private boolean serialized;
    /**
     * The type of this argument. This is used when json objects come from Rust, in order to be mapped to proper Java Objects.
     */
    private String className;

    public InvocationArg(String className, Instance instance) {
        this.json = null;
        this.className = className;
        this.instance = instance;
        this.serialized = false;
    }

    public InvocationArg(Instance instance) {
        this.json = null;
        this.className = instance.getClass().getName();
        this.instance = instance;
        this.serialized = false;
    }

    public InvocationArg(String className, String json) {
        this.instance = null;
        this.className = className;
        this.json = json;
        this.serialized = true;
    }

    public InvocationArg(String className, Object object) throws ClassNotFoundException {
        this.instance = new JsonInvocationImpl(object, Utils.forNameEnhanced(className));
        this.className = className;
        this.json = null;
        this.serialized = false;
    }

    /**
     * If true, the argument is taken straight by the Java code as Object. If
     * false, the argument is a json document that need to be deserialized to an
     * Object.
     *
     * @return The The argFrom
     */
    public boolean isSerialized() {
        return serialized;
    }

    /**
     * The type of this argument. This is used when json objects come from Rust, in order to be mapped to proper Java Objects.
     *
     * @return The classname
     */
    public String getClassName() {
        return className;
    }

    public Instance getInstance() {
        if (isSerialized()) {
            throw new InvalidArgumentException("This InvocationArg of class " + className + " is created by Rust code.");
        }
        return instance;
    }

    public String getJson() {
        if (!isSerialized()) {
            throw new InvalidArgumentException("This InvocationArg of class " + className + " is created by Java code.");
        }
        return json;
    }

    @Override
    public String toString() {
        return "classname:" + this.className + ", serialized:" + this.serialized + ", json:" + this.json + ", instance:" + this.instance;
    }

    @Override
    public Object getObject() {
        return getInstance() != null ? getInstance().getObject() : null;
    }

    @Override
    public Class<?> getObjectClass() {
        return getInstance() != null ? getInstance().getObjectClass() : null;
    }

    @Override
    public Instance invoke(String methodName, InvocationArg... args) {
        return getInstance() != null ? getInstance().invoke(methodName, args) : null;
    }

    @Override
    public Instance invokeStatic(String methodName, InvocationArg... args) {
        return getInstance() != null ? getInstance().invokeStatic(methodName, args) : null;
    }

    @Override
    public void invokeAsync(long functionPointerAddress, String methodName, InvocationArg... args) {
        if (getInstance() != null) {
            getInstance().invokeAsync(functionPointerAddress, methodName, args);
        }
    }

    @Override
    public void invokeToChannel(long channelAddress, String methodName, InvocationArg... args) {
        if (getInstance() != null) {
            getInstance().invokeToChannel(channelAddress, methodName, args);
        }
    }

    @Override
    public void initializeCallbackChannel(long channelAddress) {
        if (getInstance() != null) {
            getInstance().initializeCallbackChannel(channelAddress);
        }
    }

    @Override
    public Instance field(String fieldName) {
        return getInstance() != null ? getInstance().field(fieldName) : null;
    }
}
