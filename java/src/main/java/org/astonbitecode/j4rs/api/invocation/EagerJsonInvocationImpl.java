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
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.value.JsonValueImpl;

public class EagerJsonInvocationImpl<T> implements NativeInvocation<T> {

    private T object;
    private Class<T> clazz;
    private JsonValue jsonValue;

    public EagerJsonInvocationImpl(T instance, Class<T> clazz) {
        this.object = instance;
        this.clazz = clazz;
        this.jsonValue = new JsonValueImpl(this.object);
    }

    @Override
    public NativeInvocation invoke(String methodName, InvocationArg... arg) {
        throw new RuntimeException("Not implemented yet. Please use the JsonInvocationImpl instead");
    }

    @Override
    public NativeInvocation invokeStatic(String methodName, InvocationArg... arg) {
        throw new RuntimeException("Not implemented yet. Please use the JsonInvocationImpl instead");
    }

    @Override
    public void invokeAsync(long functionPointer, String methodName, InvocationArg... args) {
        throw new RuntimeException("Not implemented yet. Please use the JsonInvocationImpl instead");
    }

    @Override
    public T getObject() {
        throw new RuntimeException("Not implemented yet. Please use the JsonInvocationImpl instead");
    }

    @Override
    public Class<?> getObjectClass() {
        throw new RuntimeException("Not implemented yet. Please use the JsonInvocationImpl instead");
    }

    @Override
    public String getJson() {
        throw new RuntimeException("Not implemented yet. Please use the JsonInvocationImpl instead");
    }
}