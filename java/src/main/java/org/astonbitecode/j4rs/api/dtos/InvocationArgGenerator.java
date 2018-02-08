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
import org.astonbitecode.j4rs.api.ObjectValue;
import org.astonbitecode.j4rs.api.value.JsonValueImpl;
import org.astonbitecode.j4rs.errors.InvalidArgumentException;
import org.astonbitecode.j4rs.utils.Defs;

import java.util.Arrays;

public class InvocationArgGenerator {
    public GeneratedArg[] generateArgObjects(InvocationArg[] args) throws Exception {
        GeneratedArg[] generatedArgArr = Arrays.stream(args).map(invArg -> {
            GeneratedArg generatedArg;
            if (invArg.getArgFrom().equals(Defs.JAVA)) {
                NativeInvocation inv = invArg.getNativeInvocation();
                generatedArg = new GeneratedArg(inv.getObjectClass(), inv.getObject());
            } else if (invArg.getArgFrom().equals(Defs.RUST)) {
                ObjectValue objValue = new JsonValueImpl(invArg.getJson(), invArg.getClassName());
                generatedArg = new GeneratedArg(objValue.getObject().getClass(), objValue.getObject());
            } else {
                throw new InvalidArgumentException("Cannot parse argument generated from " + invArg.getArgFrom());
            }
            return generatedArg;
        }).toArray(i -> new GeneratedArg[i]);

        return generatedArgArr;
    }

    public static GeneratedArg argOf(Class clazz, Object object) {
        return new GeneratedArg(clazz, object);
    }


}
