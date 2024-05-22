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

import java.lang.reflect.Type;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.ServiceLoader;

import org.astonbitecode.j4rs.api.Instance;
import org.astonbitecode.j4rs.api.services.delegates.InstanceGeneratorDelegate;
import org.astonbitecode.j4rs.errors.InvocationException;

public class InstanceGenerator {
    private static Map<String, InstanceGeneratorDelegate> delegates = new HashMap<>();
    static {
        ServiceLoader<InstanceGeneratorDelegate> loader = ServiceLoader.load(InstanceGeneratorDelegate.class);
        Iterator<InstanceGeneratorDelegate> discoveredDelegates = loader.iterator();
        while (discoveredDelegates.hasNext()) {
            InstanceGeneratorDelegate d = discoveredDelegates.next();
            delegates.put(d.getClass().getCanonicalName(), d);
        }
    }

    public static <T> Instance<T> create(T instance, Class<T> clazz, List<Type> classGenTypes) {
        JsonInvocationImpl<T> jsonInvocation = new JsonInvocationImpl<T>(instance, clazz, classGenTypes);
        if (shouldRunInFxThread(jsonInvocation.getObjectClass())) {
            return getProxiedForJavaFx(jsonInvocation);
        } else {
            return jsonInvocation;
        }
    }

    public static <T> Instance<T> create(T instance, Class clazz) {
        JsonInvocationImpl<T> jsonInvocation = new JsonInvocationImpl<T>(instance, clazz);
        if (shouldRunInFxThread(jsonInvocation.getObjectClass())) {
            return getProxiedForJavaFx(jsonInvocation);
        } else {
            return jsonInvocation;
        }
    }

    public static <T> Instance<T> create(Class clazz) {
        JsonInvocationImpl<T> jsonInvocation = new JsonInvocationImpl(clazz);
        if (shouldRunInFxThread(jsonInvocation.getObjectClass())) {
            return getProxiedForJavaFx(jsonInvocation);
        } else {
            return jsonInvocation;
        }
    }

    private static <T> Instance<T> getProxiedForJavaFx(Instance<T> instance) {
        InstanceGeneratorDelegate delegate = delegates
                .get("org.astonbitecode.j4rs.api.invocation.JavaFxInstanceGeneratorDelegate");
        if (delegate == null) {
            throw new InvocationException(
                    "Attempted to proxy Instance in order to be executed in FX thread, but delegate is not configured. Please make sure you have j4rs-javafx in the classpath");
        } else {
            return delegate.proxy(instance);
        }
    }

    private static boolean shouldRunInFxThread(Class<?> clazz) {
        String className = clazz.getName();
        return className.startsWith("javafx") || (className.startsWith("org.astonbitecode.j4rs.api.jfx")
                && !className.startsWith("org.astonbitecode.j4rs.api.jfx.FxApplication"));
    }
}
