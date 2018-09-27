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
package org.astonbitecode.j4rs.api;

import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.invocation.JsonInvocationImpl;
import org.astonbitecode.j4rs.errors.InvocationException;

public interface NativeInvocation<T> extends ObjectValue, JsonValue {
    /**
     * Invokes a method of the instance of the class that is set for this {@link NativeInvocation}
     *
     * @param methodName The method name
     * @param args       The arguments to use for invoking the method
     * @return A {@link NativeInvocation} instance containing the result of the invocation
     */
    NativeInvocation invoke(String methodName, InvocationArg... args);

    /**
     * Invokes a static method of the class that is set for this {@link NativeInvocation}
     *
     * @param methodName The static method name
     * @param args       The arguments to use for invoking the static method
     * @return A {@link NativeInvocation} instance containing the result of the invocation
     */
    NativeInvocation invokeStatic(String methodName, InvocationArg... args);

    /**
     * Invokes asynchronously a method of the instance of the class that is set for this {@link NativeInvocation}.
     * The result of the invocation should be provided later using the performCallback method of a {@link org.astonbitecode.j4rs.api.invocation.NativeCallbackSupport} class.
     * Any possible returned objects from the actual synchronous invocation of the defined method will be dropped.
     *
     * @param functionPointerAddress The address of the function pointer that will be used later in the native side in order to actually paerform the callback.
     * @param methodName             The method name
     * @param args                   The arguments to use when invoking the callback method (the functionPointer)
     */
    void invokeAsync(long functionPointerAddress, String methodName, InvocationArg... args);

    /**
     * Invokes a method of the instance of the class that is set for this {@link NativeInvocation}.
     * The result of the invocation should be provided later using the performCallbackToChannel method of a {@link org.astonbitecode.j4rs.api.invocation.NativeCallbackToRustChannelSupport} class.
     * Any possible returned objects from the actual synchronous invocation of the defined method will be dropped.
     *
     * @param channelAddress
     * @param methodName
     * @param args
     */
    void invokeToChannel(long channelAddress, String methodName, InvocationArg... args);

    /**
     * Casts a the object that is contained in a NativeInvocation to an object of class clazz.
     *
     * @param <T>     Generically defined return type
     * @param from    The {@link NativeInvocation} to cast.
     * @param toClass The class that the provided {@link NativeInvocation} should be casted to
     * @return A {@link NativeInvocation} instance containing the result of the cast.
     */
    static <T> NativeInvocation cast(NativeInvocation from, String toClass) {
        try {
            Class<T> clazz = (Class<T>) Class.forName(toClass);
            return new JsonInvocationImpl(clazz.cast(from.getObject()), clazz);
        } catch (Exception error) {
            throw new InvocationException("Cannot cast instance of " + from.getObject().getClass().getName() + " to " + toClass, error);
        }
    }
}
