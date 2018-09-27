package org.astonbitecode.j4rs.rust;

public class RustPointer {
    private long address;

    public RustPointer(long address) {
        this.address = address;
    }

    public long getAddress() {
        return address;
    }
}
