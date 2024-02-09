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
package org.astonbitecode.j4rs.utils;

import org.astonbitecode.j4rs.api.dtos.GeneratedArg;

import java.util.Arrays;

public class Utils {

    private static boolean IsAndroid;

    static {
        try {
            Class.forName("android.os.Build");
            IsAndroid = true;
        } catch (ClassNotFoundException e) {
            IsAndroid = false;
        }
    }

    public static Class<?> forNameEnhanced(final String className) throws ClassNotFoundException {
        switch (className) {
        case "boolean":
            return boolean.class;
        case "byte":
            return byte.class;
        case "short":
            return short.class;
        case "int":
            return int.class;
        case "long":
            return long.class;
        case "float":
            return float.class;
        case "double":
            return double.class;
        case "char":
            return char.class;
        case "void":
            return void.class;
        default:
            if (!IsAndroid) {
                return Class.forName(className, true, ClassLoader.getSystemClassLoader());
            } else {
                return Class.forName(className);
            }
        }
    }

    // Return one of the classes of the GeneratedArgs.
    // Currently there is no need to support many classes.
    // In the future, we may need to converge to the common parent of all the
    // GeneratedArgs.
    public static Class<?> forNameBasedOnArgs(final GeneratedArg[] params) {
        return Arrays.stream(params).map(arg -> arg.getClazz()).reduce((a, b) -> a).orElse(Void.class);
    }

    // Converts primitive types to their wrapper class.  Useful in matching method parameters.
    public static Class<?> toWrapper(Class<?> clazz) {
        if (!clazz.isPrimitive())
            return clazz;

        if (clazz == Integer.TYPE)
            return Integer.class;
        if (clazz == Long.TYPE)
            return Long.class;
        if (clazz == Boolean.TYPE)
            return Boolean.class;
        if (clazz == Byte.TYPE)
            return Byte.class;
        if (clazz == Character.TYPE)
            return Character.class;
        if (clazz == Float.TYPE)
            return Float.class;
        if (clazz == Double.TYPE)
            return Double.class;
        if (clazz == Short.TYPE)
            return Short.class;
        if (clazz == Void.TYPE)
            return Void.class;

        return clazz;
    }
}
