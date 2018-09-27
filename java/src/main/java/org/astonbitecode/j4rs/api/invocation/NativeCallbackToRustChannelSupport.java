package org.astonbitecode.j4rs.api.invocation;

import org.astonbitecode.j4rs.api.NativeInvocation;
import org.astonbitecode.j4rs.errors.InvocationException;
import org.astonbitecode.j4rs.rust.RustPointer;

import java.util.Optional;

/**
 * Performs native callbacks to Rust channels
 */
public class NativeCallbackToRustChannelSupport {
    private static native int docallbacktochannel(long channelPointerAddress, NativeInvocation inv);

    private Optional<RustPointer> channelPointerOpt = Optional.empty();

    static void initialize(String libname) throws UnsatisfiedLinkError {
        String libpath = System.getProperty("java.library.path");
        System.out.println("Loading native library " + libname + " from path " + libpath);
        System.loadLibrary(libname);
    }

    /**
     * Perform a callback
     *
     * @param obj The {@link Object} to pass in the callback.
     */
    protected void doCallback(Object obj) {
        if (channelPointerOpt.isPresent() && obj != null) {
            docallbacktochannel(channelPointerOpt.get().getAddress(), new JsonInvocationImpl(obj, obj.getClass()));
        } else {
            throw new InvocationException("Cannot do callback. Please make sure that you don't try to access this method while being in the constructor of your class (that extends NativeCallbackSupport)");
        }
    }

    final void initPointer(RustPointer p) {
        this.channelPointerOpt = Optional.of(p);
    }
}
