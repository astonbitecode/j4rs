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
package org.astonbitecode.j4rs.api.invocation;

import org.astonbitecode.j4rs.api.JsonValue;
import org.astonbitecode.j4rs.api.NativeInvocation;
import org.astonbitecode.j4rs.api.dtos.GeneratedArg;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.dtos.InvocationArgGenerator;
import org.astonbitecode.j4rs.api.value.JsonValueImpl;
import org.astonbitecode.j4rs.errors.InvocationException;
import org.astonbitecode.j4rs.rust.FunctionPointer;

import java.lang.reflect.Method;
import java.util.Arrays;

public class JsonInvocationImpl<T> implements NativeInvocation<T> {

    private T object;
    private Class<T> clazz;
    private InvocationArgGenerator gen = new InvocationArgGenerator();

    public JsonInvocationImpl(Class<T> clazz) {
        this.object = null;
        this.clazz = clazz;
    }

    public JsonInvocationImpl(T instance, Class<T> clazz) {
        this.object = instance;
        this.clazz = clazz;
    }

    @Override
    public NativeInvocation invoke(String methodName, InvocationArg... args) {
        // Check the existence of the instance to invoke
        if (this.object == null) {
            throw new InvocationException("Cannot invoke the class " + this.clazz.getName() + ". It is not instantiated yet.");
        }
        // Invoke the instance
        try {
            CreatedInstance createdInstance = invokeMethod(methodName, gen.generateArgObjects(args));
            return new JsonInvocationImpl(createdInstance.object, createdInstance.clazz);
        } catch (Exception error) {
            throw new InvocationException("While invoking method " + methodName + " of Class " + this.clazz.getName(), error);
        }
    }

    @Override
    public NativeInvocation invokeStatic(String methodName, InvocationArg... args) {
        try {
            CreatedInstance createdInstance = invokeMethod(methodName, gen.generateArgObjects(args));
            return new JsonInvocationImpl(createdInstance.object, createdInstance.clazz);
        } catch (Exception error) {
            throw new InvocationException("Error while invoking method " + methodName + " of Class " + this.clazz.getName(), error);
        }
    }

    @Override
    public void invokeAsync(long functionPointerAddress, String methodName, InvocationArg... args) {
        // Check that the class of the invocation extends the NativeCallbackSupport
        if (!NativeCallbackSupport.class.isAssignableFrom(this.clazz)) {
            throw new InvocationException("Cannot invoke asynchronously the class " + this.clazz.getName() + ". The class does not extend the class " + NativeCallbackSupport.class.getName());
        } else {
            // Initialize the pointer
            ((NativeCallbackSupport) object).initPointer(new FunctionPointer(functionPointerAddress));
            // Invoke (any possible returned objects will be dropped)
            invoke(methodName, args);
        }
    }

    @Override
    public T getObject() {
        return object;
    }

    @Override
    public Class<T> getObjectClass() {
        return clazz;
    }

    @Override
    public String getJson() {
        JsonValue jsonValue = new JsonValueImpl(this.object);
        return jsonValue.getJson();
    }

    CreatedInstance invokeMethod(String methodName, GeneratedArg[] generatedArgs) throws Exception {
        Class[] argTypes = Arrays.stream(generatedArgs)
                .map(invGeneratedArg -> {
                    try {
                        return invGeneratedArg.getClazz();
                    } catch (Exception error) {
                        throw new InvocationException("Cannot parse the parameter types while invoking method", error);
                    }
                })
                .toArray(size -> new Class[size]);
        Object[] argObjects = Arrays.stream(generatedArgs)
                .map(invGeneratedArg -> {
                    try {
                        return invGeneratedArg.getObject();
                    } catch (Exception error) {
                        throw new InvocationException("Cannot parse the parameter objects while invoking method", error);
                    }
                })
                .toArray(size -> new Object[size]);

        Method methodToInvoke = this.clazz.getDeclaredMethod(methodName, argTypes);

        Class<?> invokedMethodReturnType = methodToInvoke.getReturnType();
        Object returnedObject = methodToInvoke.invoke(this.object, argObjects);
        return new CreatedInstance(invokedMethodReturnType, returnedObject);
    }

    class CreatedInstance {
        private Class clazz;
        private Object object;

        public CreatedInstance(Class clazz, Object object) {
            this.clazz = clazz;
            this.object = object;
        }

        public Class getClazz() {
            return clazz;
        }

        public Object getObject() {
            return object;
        }
    }
}
