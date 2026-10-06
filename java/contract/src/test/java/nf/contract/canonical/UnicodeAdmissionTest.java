package nf.contract.canonical;

import nf.contract.Unicode13;

public final class UnicodeAdmissionTest {
    private UnicodeAdmissionTest() { }
    public static void main(String[] args) {
        Canonical.validateText("café 🚀", 4096);
        for (int scalar : new int[] {0x105d2, 0x105c9, 0x1fae0, 0xffff, 0x10ffff}) {
            if (Unicode13.contains(scalar)) throw new AssertionError("Post-profile or noncharacter scalar admitted");
            try { Canonical.validateText(new String(Character.toChars(scalar)), 4096); throw new AssertionError("Unsupported scalar encoded"); }
            catch (Validation expected) { }
        }
        String todhri = new String(Character.toChars(0x105d2)) + "\u0307";
        try { Canonical.validateText(todhri, 4096); throw new AssertionError("Version-dependent NFC admitted"); } catch (Validation expected) { }
        if (!Unicode13.contains(0x1f680) || !Unicode13.contains(0x10fffd) || Unicode13.contains(-1) || Unicode13.contains(0x110000)) throw new AssertionError("Scalar profile boundary");
        System.out.println("PASS: pinned Unicode13 admission rejects runtime-normalizer version drift");
    }
}
