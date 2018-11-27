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

import org.astonbitecode.j4rs.api.NativeInvocation;
import org.astonbitecode.j4rs.errors.InvocationException;
import org.astonbitecode.j4rs.rust.RustPointer;

import java.util.Optional;

/**
 * Performs native callbacks to Rust
 */
public class NativeCallbackSupport {
    private static native void docallback(long functionPointerAddress, NativeInvocation inv);

    private Optional<RustPointer> functionPointerOpt = Optional.empty();

    static void initialize(String libname) throws UnsatisfiedLinkError {
        System.loadLibrary(libname);
    }

    /**
     * Perform a callback
     *
     * @param obj The {@link Object} to pass in the callback.
     */
    protected void doCallback(Object obj) {
        if (functionPointerOpt.isPresent() && obj != null) {
            docallback(functionPointerOpt.get().getAddress(), new JsonInvocationImpl(obj, obj.getClass()));
        } else {
            throw new InvocationException("Cannot do callback. Please make sure that you don't try to access this method while being in the constructor of your class (that extends NativeCallbackSupport)");
        }
    }

    final void initPointer(RustPointer p) {
        this.functionPointerOpt = Optional.of(p);
    }
}
