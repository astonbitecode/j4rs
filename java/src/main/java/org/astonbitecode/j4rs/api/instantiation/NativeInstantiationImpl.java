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

import org.astonbitecode.j4rs.api.Instance;
import org.astonbitecode.j4rs.api.dtos.GeneratedArg;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.dtos.InvocationArgGenerator;
import org.astonbitecode.j4rs.api.invocation.InstanceGenerator;
import org.astonbitecode.j4rs.api.invocation.JsonInvocationImpl;
import org.astonbitecode.j4rs.errors.InstantiationException;
import org.astonbitecode.j4rs.utils.Utils;

import java.lang.reflect.*;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;

public class NativeInstantiationImpl {
    static InvocationArgGenerator gen = new InvocationArgGenerator();

    public static Instance instantiate(String className, InvocationArg... args) {
        try {
            CreatedInstance createdInstance = createInstance(className, generateArgObjects(args));
            return InstanceGenerator.create(createdInstance.object, createdInstance.clazz);
        } catch (Exception error) {
            throw new InstantiationException("Cannot create instance of " + className, error);
        }
    }

    public static Instance createForStatic(String className) {
        try {
            Class<?> clazz = Utils.forNameEnhanced(className);
            return new JsonInvocationImpl(clazz);
        } catch (Exception error) {
            throw new InstantiationException("Cannot create instance of " + className, error);
        }
    }

    public static Instance createJavaArray(String className, InvocationArg... args) {
        try {
            CreatedInstance createdInstance = createCollection(className, generateArgObjects(args), J4rsCollectionType.Array);
            return new JsonInvocationImpl(createdInstance.object, createdInstance.clazz);
        } catch (Exception error) {
            throw new InstantiationException("Cannot create Java Array of " + className, error);
        }
    }

    public static Instance createJavaList(String className, InvocationArg... args) {
        try {
            CreatedInstance createdInstance = createCollection(className, generateArgObjects(args), J4rsCollectionType.List);
            return new JsonInvocationImpl(createdInstance.object, createdInstance.clazz);
        } catch (Exception error) {
            throw new InstantiationException("Cannot create Java List of " + className, error);
        }
    }

    static GeneratedArg[] generateArgObjects(InvocationArg[] args) throws Exception {
        return gen.generateArgObjects(args);
    }

    static CreatedInstance createInstance(String className, GeneratedArg[] params) throws Exception {
        Class<?> clazz = Utils.forNameEnhanced(className);
        Class<?>[] paramTypes = Arrays.stream(params).map(param -> param.getClazz())
                .toArray(size -> new Class<?>[size]);
        Object[] paramObjects = Arrays.stream(params).map(param -> param.getObject())
                .toArray(size -> new Object[size]);
        Constructor<?> constructor = findConstructor(clazz, paramTypes);
        Object instance = constructor.newInstance(paramObjects);
        return new CreatedInstance(clazz, instance);
    }

    private static Constructor<?> findConstructor(Class clazz, Class[] argTypes) throws NoSuchMethodException {
        List<Constructor> found = Arrays.stream(clazz.getConstructors())
                // Match the params number
                .filter(constructor -> constructor.getGenericParameterTypes().length == argTypes.length)
                .filter(constructor -> {
                    // Each element of the matchedParams list shows whether a parameter is matched or not
                    List<Boolean> matchedParams = new ArrayList<>();
                    // Get the parameter types of the method to check if matches
                    Type[] pts = constructor.getGenericParameterTypes();
                    for (int i = 0; i < argTypes.length; i++) {
                        // Check each parameter type
                        Type typ = pts[i];

                        if (typ instanceof ParameterizedType || typ instanceof WildcardType) {
                            // For generic parameters, the type erasure makes the parameter be an Object.class
                            // Therefore, the argument is always matched
                            matchedParams.add(true);
                        } else if (typ instanceof GenericArrayType) {
                            // TODO: Improve by checking the actual types of the arrays?
                            matchedParams.add(argTypes[i].isArray());
                        } else if (typ instanceof Class) {
                            // In case of TypeVariable, the arg matches via the equals method
                            matchedParams.add(((Class<?>) typ).isAssignableFrom(argTypes[i]));
                        } else {
                            // We get to this point if the TypeVariable is a generic, which is defined with a name like T, U etc.
                            // The type erasure makes the parameter be an Object.class. Therefore, the argument is always matched.
                            // TODO:
                            // We may have some info about the generic types (if they are defined in the Class scope).
                            // Can we use this info to provide some type safety? Use matchedParams.add(validateSomeTypeSafety(argTypes[i]));
                            // In that case however, we don't catch the situation where a class is defined with a generic T in the class scope,
                            // but there is a method that defines another generic U in the method scope.
                            matchedParams.add(true);
                        }
                    }
                    return matchedParams.stream().allMatch(Boolean::booleanValue);
                })
                .collect(Collectors.toList());
        if (!found.isEmpty()) {
            return found.get(0);
        } else {
            Class<?> superclass = clazz.getSuperclass();
            if (superclass == null) {
                throw new NoSuchMethodException("Constructor was not found in " + clazz.getName() + " or its ancestors.");
            }
            return findConstructor(superclass, argTypes);
        }
    }

    static CreatedInstance createCollection(String className, GeneratedArg[] params, J4rsCollectionType collectionType) throws Exception {
        boolean isJ4rsArray = className.equals(InvocationArg.CONTENTS_ARRAY);
        Class<?> clazz = isJ4rsArray ? Utils.forNameBasedOnArgs(params) : Utils.forNameEnhanced(className);
        Object arrayObj = Array.newInstance(clazz, params.length);

        Class<?>[] paramTypes = Arrays.stream(params).map(param -> param.getClazz())
                .toArray(size -> new Class<?>[size]);

        if (!isJ4rsArray && !Arrays.stream(paramTypes).allMatch(type -> type.getName().equals(className))) {
            throw new IllegalArgumentException("Could not create Java array. All the arguments should be of class " + className);
        }

        Object[] paramObjects = Arrays.stream(params).map(param -> param.getObject())
                .toArray(size -> new Object[size]);

        for (int i = 0; i < params.length; i++) {
            Array.set(arrayObj, i, paramObjects[i]);
        }

        switch (collectionType) {
            case Array:
                return new CreatedInstance(arrayObj.getClass(), arrayObj);
            case List: {
                Object l = clazz.isPrimitive() ? arrayObj : Arrays.asList(((Object[]) arrayObj));
                return new CreatedInstance(l.getClass(), l);
            }
            default:
                return new CreatedInstance(arrayObj.getClass(), arrayObj);
        }

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

    enum J4rsCollectionType {
        Array,
        List
    }
}
