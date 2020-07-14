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

import org.astonbitecode.j4rs.api.invocation.JsonInvocationImpl;
import org.astonbitecode.j4rs.errors.InvalidArgumentException;
import org.astonbitecode.j4rs.utils.Dummy;
import org.junit.Test;

public class InvocationArgTest {
    private static String CLASS_NAME = "a.class.Name";

    @Test(expected = InvalidArgumentException.class)
    public void getNativeInvocationOnAnArgCreatedByRust() {
        InvocationArg ia = new InvocationArg(CLASS_NAME, "{\"a\":\"b\"}");
        ia.getInstance();
    }

    @Test(expected = InvalidArgumentException.class)
    public void getNativeInvocationOnAnArgCreatedByJava() {
        InvocationArg ia = new InvocationArg(CLASS_NAME, new JsonInvocationImpl(new Dummy(), Dummy.class));
        ia.getJson();
    }

    @Test
    public void getArgFrom() {
        InvocationArg ia1 = new InvocationArg(CLASS_NAME, "{\"a\":\"b\"}");
        assert ia1.isSerialized();
        assert ia1.getClassName().equals(CLASS_NAME);

        InvocationArg ia2 = new InvocationArg(CLASS_NAME, new JsonInvocationImpl(new Dummy(), Dummy.class));
        assert !ia2.isSerialized();
        assert ia2.getClassName().equals(CLASS_NAME);
    }
}
