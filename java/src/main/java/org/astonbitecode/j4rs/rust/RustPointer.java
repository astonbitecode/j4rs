package org.astonbitecode.j4rs.rust;

public class RustPointer {
    private Long address;

    public RustPointer(long address) {
        this.address = address;
    }

    public Long getAddress() {
        return address;
    }
}
