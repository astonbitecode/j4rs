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

import org.astonbitecode.j4rs.api.Instance;
import org.astonbitecode.j4rs.api.JsonValue;
import org.astonbitecode.j4rs.api.dtos.GeneratedArg;
import org.astonbitecode.j4rs.api.dtos.InvocationArg;
import org.astonbitecode.j4rs.api.dtos.InvocationArgGenerator;
import org.astonbitecode.j4rs.api.value.JsonValueFactory;
import org.astonbitecode.j4rs.errors.InvocationException;
import org.astonbitecode.j4rs.rust.RustPointer;

import java.lang.reflect.Field;
import java.lang.reflect.GenericArrayType;
import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.lang.reflect.WildcardType;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.stream.Collectors;

public class JsonInvocationImpl<T> implements Instance<T> {

    // The instance that this JsonInvocationImpl holds
    private T object;
    // The class of the instance that this JsonInvocationImpl holds
    private Class<T> clazz;
    // A list of Types that may have been defined for the instance that this JsonInvocationImpl holds.
    // It is non empty in the case that the instance that this JsonInvocationImpl holds is created using generics.
    private List<Type> classGenTypes = new ArrayList<>();
    private InvocationArgGenerator gen = new InvocationArgGenerator();

    public JsonInvocationImpl(Class<T> clazz) {
        this.object = null;
        this.clazz = clazz;
    }

    public JsonInvocationImpl(T instance, Class<T> clazz) {
        this.object = instance;
        this.clazz = clazz;
    }

    public JsonInvocationImpl(T instance, Class<T> clazz, List<Type> classGenTypes) {
        this.object = instance;
        this.clazz = clazz;
        this.classGenTypes = classGenTypes;
    }

    @Override
    public Instance invoke(String methodName, InvocationArg... args) {
        // Invoke the instance
        try {
            CreatedInstance createdInstance = invokeMethod(methodName, gen.generateArgObjects(args));
            return InstanceGenerator.create(createdInstance.object, createdInstance.clazz, createdInstance.classGenTypes);
        } catch (Exception error) {
            throw new InvocationException("While invoking method " + methodName + " of Class " + this.clazz.getName(), error);
        }
    }

    @Override
    public Instance invokeStatic(String methodName, InvocationArg... args) {
        try {
            CreatedInstance createdInstance = invokeMethod(methodName, gen.generateArgObjects(args));
            return InstanceGenerator.create(createdInstance.object, createdInstance.clazz, createdInstance.classGenTypes);
        } catch (Exception error) {
            throw new InvocationException("Error while invoking method " + methodName + " of Class " + this.clazz.getName(), error);
        }
    }

    @Override
    public void invokeAsync(long functionPointerAddress, String methodName, InvocationArg... args) {
        // Check that the class of the invocation extends the NativeCallbackSupport
        if (!NativeCallbackSupport.class.isAssignableFrom(this.clazz)) {
            throw new InvocationException("Cannot invoke asynchronously the class " + this.clazz.getName() + ". The class does not extend the class " + NativeCallbackSupport.class.getName());
        } else {
            // Initialize the pointer
            ((NativeCallbackSupport) object).initPointer(new RustPointer(functionPointerAddress));
            // Invoke (any possible returned objects will be dropped)
            invoke(methodName, args);
        }
    }

    @Override
    public void invokeToChannel(long channelAddress, String methodName, InvocationArg... args) {
        initializeCallbackChannel(channelAddress);
        invoke(methodName, args);
    }

    @Override
    public void initializeCallbackChannel(long channelAddress) {
        // Check that the class of the invocation extends the NativeCallbackToRustChannelSupport
        if (!NativeCallbackToRustChannelSupport.class.isAssignableFrom(this.clazz)) {
            throw new InvocationException("Cannot initialize callback channel for class " + this.clazz.getName() + ". The class does not extend the class " + NativeCallbackToRustChannelSupport.class.getName());
        } else {
            // Initialize the pointer
            ((NativeCallbackToRustChannelSupport) object).initPointer(new RustPointer(channelAddress));
        }
    }

    @Override
    public Instance field(String fieldName) {
        try {
            CreatedInstance createdInstance = getField(fieldName);
            return new JsonInvocationImpl(createdInstance.object, createdInstance.clazz);
        } catch (Exception error) {
            throw new InvocationException("Error while accessing field " + fieldName + " of Class " + this.clazz.getName(), error);
        }
    }

    @Override
    public T getObject() {
        return object;
    }

    @Override
    public Class<T> getObjectClass() {
        return clazz;
    }

    @Override
    public String getObjectClassName() {
        return clazz != null ? clazz.getName() : "null";
    }

    @Override
    public String getJson() {
        JsonValue jsonValue = JsonValueFactory.create(this.object);
        return jsonValue.getJson();
    }

    CreatedInstance getField(String fieldName) throws Exception {
        Field field = this.clazz.getField(fieldName);
        Object fieldObject = field.get(this.object);
        return new CreatedInstance(field.getType(), fieldObject);
    }

    CreatedInstance invokeMethod(String methodName, GeneratedArg[] generatedArgs) throws Exception {
        Class[] argTypes = Arrays.stream(generatedArgs)
                .map(invGeneratedArg -> {
                    try {
                        return invGeneratedArg.getClazz();
                    } catch (Exception error) {
                        throw new InvocationException("Cannot parse the parameter types while invoking method", error);
                    }
                })
                .toArray(size -> new Class[size]);
        Object[] argObjects = Arrays.stream(generatedArgs)
                .map(invGeneratedArg -> {
                    try {
                        return invGeneratedArg.getObject();
                    } catch (Exception error) {
                        throw new InvocationException("Cannot parse the parameter objects while invoking method", error);
                    }
                })
                .toArray(size -> new Object[size]);

        Method methodToInvoke = findMethodInHierarchy(this.clazz, methodName, argTypes);
        List<Type> retClassGenTypes = new ArrayList<>();

        Type returnType = methodToInvoke.getGenericReturnType();
        if (returnType instanceof ParameterizedType) {
            ParameterizedType type = (ParameterizedType) returnType;
            retClassGenTypes = Arrays.asList(type.getActualTypeArguments());
        }

        Class<?> invokedMethodReturnType = methodToInvoke.getReturnType();
        Object returnedObject = methodToInvoke.invoke(this.object, argObjects);
        return new CreatedInstance(invokedMethodReturnType, returnedObject, retClassGenTypes);
    }

    Method findMethodInHierarchy(Class clazz, String methodName, Class[] argTypes) throws NoSuchMethodException {
        // Get the declared and methods defined in the interfaces of the class.
        Set<Method> methods = new HashSet<>(Arrays.asList(clazz.getDeclaredMethods()));
        Set<Method> interfacesMethods = Arrays.stream(clazz.getInterfaces())
                .map(c -> c.getDeclaredMethods())
                .flatMap(m -> Arrays.stream(m))
                .collect(Collectors.toSet());
        methods.addAll(interfacesMethods);

        List<Method> found = methods.stream()
                // Match the method name
                .filter(m -> m.getName().equals(methodName))
                // Match the params number
                .filter(m -> m.getGenericParameterTypes().length == argTypes.length)
                // Match the actual parameters
                .filter(m -> {
                    // Each element of the matchedParams list shows whether a parameter is matched or not
                    List<Boolean> matchedParams = new ArrayList<>();

                    // Get the parameter types of the method to check if matches
                    Type[] pts = m.getGenericParameterTypes();
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
                throw new NoSuchMethodException("Method " + methodName + " was not found in " + this.clazz.getName() + " or its ancestors.");
            }
            return findMethodInHierarchy(superclass, methodName, argTypes);
        }
    }

    private boolean validateSomeTypeSafety(Class c) {
        List<Type> filteredTypeList = this.classGenTypes.stream()
                .filter(cgt -> ((Class) cgt).isAssignableFrom(c))
                .collect(Collectors.toList());
        // If ClassGenTypes exist, the class c should be one of them
        return this.classGenTypes.isEmpty() || filteredTypeList.isEmpty();
    }

    class CreatedInstance {
        private Class clazz;
        private Object object;
        private List<Type> classGenTypes;

        public CreatedInstance(Class clazz, Object object) {
            this.clazz = clazz;
            this.object = object;
        }

        public CreatedInstance(Class clazz, Object object, List<Type> classGenTypes) {
            this.clazz = clazz;
            this.object = object;
            this.classGenTypes = classGenTypes;
        }

        public Class getClazz() {
            return clazz;
        }

        public Object getObject() {
            return object;
        }
    }


}
