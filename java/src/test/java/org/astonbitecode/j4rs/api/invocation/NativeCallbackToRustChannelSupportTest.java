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

import org.astonbitecode.j4rs.errors.InvocationException;
import org.astonbitecode.j4rs.rust.RustPointer;
import org.junit.Test;
import org.mockito.Mockito;

import static org.mockito.Mockito.mock;

public class NativeCallbackToRustChannelSupportTest {

    @Test(expected = InvocationException.class)
    public void invokeBeforeInitialized() {
        class Dummy extends NativeCallbackToRustChannelSupport {
            public Dummy() {
                super.doCallback("");
            }
        }

        new Dummy();
    }

    @Test(expected = UnsatisfiedLinkError.class)
    public void invokeSuccess() {
        RustPointer fp = mock(RustPointer.class);
        class Dummy extends NativeCallbackToRustChannelSupport {
        }

        Dummy d = new Dummy();
        Dummy spied = Mockito.spy(d);

        spied.initPointer(fp);
        // Here we will get an UnsatisfiedLinkError since the native libs are not
        // initialized in the tests
        spied.doCallback("");
    }

}
