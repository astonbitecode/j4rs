/*
 * Copyright 2019 astonbitecode
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

import org.astonbitecode.j4rs.api.Instance;
import org.astonbitecode.j4rs.errors.InvocationException;
import org.astonbitecode.j4rs.rust.RustPointer;

import java.util.Optional;

/**
 * Performs native callbacks to Rust channels
 */
public class NativeCallbackToRustChannelSupport {
    private static native int docallbacktochannel(long channelPointerAddress, Instance inv);

    private Optional<RustPointer> channelPointerOpt = Optional.empty();

    static void initialize(String libname) {
        try {
            System.loadLibrary(libname);
        } catch (UnsatisfiedLinkError error) {
            System.err.println(
                    "The Callbacks are not initialized because the j4rs lib was not found. You may ignore this error if you don't use callbacks.");
            error.printStackTrace();
        }
    }

    /**
     * Perform a callback
     *
     * @param obj The {@link Object} to pass in the callback.
     */
    public void doCallback(Object obj) {
        if (channelPointerOpt.isPresent() && obj != null) {
            docallbacktochannel(channelPointerOpt.get().getAddress(), InstanceGenerator.create(obj, obj.getClass()));
        } else {
            throw new InvocationException(
                    "Cannot do callback. Please make sure that you don't try to access this method while being in the constructor of your class (that extends NativeCallbackSupport)");
        }
    }

    final void initPointer(RustPointer p) {
        this.channelPointerOpt = Optional.of(p);
    }
}
