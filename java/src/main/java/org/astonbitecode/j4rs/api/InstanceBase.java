package org.astonbitecode.j4rs.api;

import org.astonbitecode.j4rs.api.invocation.JsonInvocationImpl;
import org.astonbitecode.j4rs.errors.InvocationException;

public class InstanceBase {
    /**
     * Casts a the object that is contained in a NativeInvocation to an object of class clazz.
     *
     * @param <T>     Generically defined return type
     * @param from    The {@link Instance} to cast.
     * @param toClass The class that the provided {@link Instance} should be casted to
     * @return A {@link Instance} instance containing the result of the cast.
     */
    public static <T> Instance cast(Instance from, String toClass) {
        try {
            Class<T> clazz = (Class<T>) Class.forName(toClass);
            return new JsonInvocationImpl(clazz.cast(from.getObject()), clazz);
        } catch (Exception error) {
            throw new InvocationException("Cannot cast instance of " + from.getObject().getClass().getName() + " to " + toClass, error);
        }
    }

    /**
     * Clones a NativeInvocation
     *
     * @param from The object to clone.
     * @param <T>  Generically defined return type
     * @return a {@link Instance} instance.
     */
    public static <T> Instance cloneInstance(Instance from) {
        return new JsonInvocationImpl(from.getObject(), from.getObjectClass());
    }
}
