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
package org.astonbitecode.j4rs.tests;

import org.astonbitecode.j4rs.api.NativeInvocation;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.instantiation.NativeInstantiationImpl;
import org.junit.Ignore;
import org.junit.Test;

import java.util.stream.IntStream;

public class MyTestTest {

    @Ignore
    public void dummy() {
        NativeInvocation instance = NativeInstantiationImpl.instantiate("org.astonbitecode.j4rs.tests.MyTest");
        IntStream.range(0, 1000000000)
                .forEach(i -> {
                    if (i % 100000 == 0) {
                        System.out.println(i);
                    }

                    InvocationArg ia = new InvocationArg("java.lang.String", "\"astring\"");
                    instance.invoke("getMyWithArgs", ia);
                });
    }
}
