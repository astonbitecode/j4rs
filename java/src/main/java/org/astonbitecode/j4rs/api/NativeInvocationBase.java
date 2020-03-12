package org.astonbitecode.j4rs.api;

import org.astonbitecode.j4rs.api.invocation.JsonInvocationImpl;
import org.astonbitecode.j4rs.errors.InvocationException;

public class NativeInvocationBase {
    /**
     * Casts a the object that is contained in a NativeInvocation to an object of class clazz.
     *
     * @param <T>     Generically defined return type
     * @param from    The {@link NativeInvocation} to cast.
     * @param toClass The class that the provided {@link NativeInvocation} should be casted to
     * @return A {@link NativeInvocation} instance containing the result of the cast.
     */
    public static <T> NativeInvocation cast(NativeInvocation from, String toClass) {
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
     * @return a {@link NativeInvocation} instance.
     */
    public static <T> NativeInvocation cloneInstance(NativeInvocation from) {
        return new JsonInvocationImpl(from.getObject(), from.getObjectClass());
    }
}
