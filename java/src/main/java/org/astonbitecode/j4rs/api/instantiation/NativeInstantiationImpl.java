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
package org.astonbitecode.j4rs.api.instantiation;

import org.astonbitecode.j4rs.api.NativeInstantiation;
import org.astonbitecode.j4rs.api.NativeInvocation;
import org.astonbitecode.j4rs.api.dtos.GeneratedArg;
import org.astonbitecode.j4rs.api.dtos.InvocationArgGenerator;
import org.astonbitecode.j4rs.api.invocation.JsonInvocationImpl;
import org.astonbitecode.j4rs.errors.InstantiationException;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;

import java.lang.reflect.Constructor;
import java.util.Arrays;

public class NativeInstantiationImpl {
    static InvocationArgGenerator gen = new InvocationArgGenerator();

    public static NativeInvocation instantiate(String className, InvocationArg... args) {
        try {
            CreatedInstance createdInstance = createInstance(className, generateArgObjects(args));
            return new JsonInvocationImpl(createdInstance.object, createdInstance.clazz);
        } catch (Exception error) {
            throw new InstantiationException("Cannot create instance of " + className, error);
        }
    }

    public static NativeInvocation createForStatic(String className) {
        try {
            Class<?> clazz = Class.forName(className);
            return new JsonInvocationImpl(clazz);
        } catch (Exception error) {
            throw new InstantiationException("Cannot create instance of " + className, error);
        }
    }

    static GeneratedArg[] generateArgObjects(InvocationArg[] args) throws Exception {
        return gen.generateArgObjects(args);
    }

    static CreatedInstance createInstance(String className, GeneratedArg[] params) throws Exception {
        Class<?> clazz = Class.forName(className);
        Class<?>[] paramTypes = Arrays.stream(params).map(param -> param.getClazz())
                .toArray(size -> new Class<?>[size]);
        Object[] paramObjects = Arrays.stream(params).map(param -> param.getObject())
                .toArray(size -> new Object[size]);
        Constructor<?> constructor = clazz.getConstructor(paramTypes);
        Object instance = constructor.newInstance(paramObjects);
        return new CreatedInstance(clazz, instance);
    }

    static class CreatedInstance {
        private Class clazz;
        private Object object;

        public CreatedInstance(Class clazz, Object object) {
            this.clazz = clazz;
            this.object = object;
        }

        public Class getClazz() {
            return clazz;
        }

        public Object getObject() {
            return object;
        }
    }
}
