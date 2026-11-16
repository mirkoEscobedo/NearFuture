package org.lwjgl.opengl;

/** Test-only public-seam fixture. Never packages a game or LWJGL implementation. */
public final class Display {
    private Display() { }
    public static int swaps;
    public static void swapBuffers() { swaps++; }
}
