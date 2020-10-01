/*
 * Copyright 2020 astonbitecode
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

import java.lang.reflect.Type;
import java.util.List;

public class InstanceGenerator {
    public static <T> Instance<T> create(T instance, Class<T> clazz, List<Type> classGenTypes) {
        JsonInvocationImpl<T> jsonInvocation = new JsonInvocationImpl<T>(instance, clazz, classGenTypes);
        if (shouldRunInFxThread(jsonInvocation.getObjectClass())) {
            return new JavaFxInvocation<T>(jsonInvocation);
        } else {
            return jsonInvocation;
        }
    }

    public static <T> Instance<T> create(T instance, Class clazz) {
        JsonInvocationImpl<T> jsonInvocation = new JsonInvocationImpl<T>(instance, clazz);
        if (shouldRunInFxThread(jsonInvocation.getObjectClass())) {
            return new JavaFxInvocation<T>(jsonInvocation);
        } else {
            return jsonInvocation;
        }
    }

    public static <T> Instance<T> create(Class clazz) {
        JsonInvocationImpl<T> jsonInvocation = new JsonInvocationImpl(clazz);
        if (shouldRunInFxThread(jsonInvocation.getObjectClass())) {
            return new JavaFxInvocation<T>(jsonInvocation);
        } else {
            return jsonInvocation;
        }
    }

    private static boolean shouldRunInFxThread(Class<?> clazz) {
        String className = clazz.getName();
        return className.startsWith("javafx") ||
                (className.startsWith("org.astonbitecode.j4rs.api.jfx") && !className.startsWith("org.astonbitecode.j4rs.api.jfx.FxApplication"));
    }
}
