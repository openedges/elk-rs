package org.eclipse.elk.graph.json.test;

import java.io.PrintStream;
import java.util.Random;

/**
 * A {@link Random} subclass that logs every call to stderr in a format
 * compatible with the Rust {@code random_trace.rs} module.
 *
 * <p>Output format (one line per call):</p>
 * <pre>
 * [random #N] method() = value @ caller_class::caller_method
 * </pre>
 *
 * <p>Activate by replacing {@code InternalProperties.RANDOM} on the LGraph
 * with an instance of this class before layout begins.</p>
 */
public class TracingRandom extends Random {
    private static final long serialVersionUID = 1L;

    /** Global counter shared across ALL TracingRandom instances (matches Rust's global atomic). */
    private static final java.util.concurrent.atomic.AtomicInteger GLOBAL_COUNTER =
            new java.util.concurrent.atomic.AtomicInteger(0);

    private final PrintStream out;

    public TracingRandom(long seed) {
        super(seed);
        this.out = System.err;
    }

    public TracingRandom(long seed, PrintStream out) {
        super(seed);
        this.out = out;
    }

    /** Reset the global counter (call once at the start of layout). */
    public static void resetGlobalCounter() {
        GLOBAL_COUNTER.set(0);
    }

    @Override
    public long nextLong() {
        long v = super.nextLong();
        trace("next_long", String.valueOf(v));
        return v;
    }

    @Override
    public boolean nextBoolean() {
        boolean v = super.nextBoolean();
        trace("next_boolean", String.valueOf(v));
        return v;
    }

    @Override
    public double nextDouble() {
        double v = super.nextDouble();
        trace("next_double", String.format("%.7f", v));
        return v;
    }

    @Override
    public float nextFloat() {
        float v = super.nextFloat();
        trace("next_float", String.format("%.7f", (double) v));
        return v;
    }

    @Override
    public int nextInt() {
        int v = super.nextInt();
        trace("next_int", String.valueOf(v));
        return v;
    }

    @Override
    public int nextInt(int bound) {
        int v = super.nextInt(bound);
        trace("next_int", String.valueOf(v));
        return v;
    }

    private void trace(String method, String value) {
        String location = findCallerLocation();
        int n = GLOBAL_COUNTER.getAndIncrement();
        out.printf("[random #%d] %s() = %s @ %s%n", n, method, value, location);
    }

    /**
     * Walk the stack to find the first frame outside TracingRandom and
     * java.util.Random, returning "ClassName::methodName".
     */
    private String findCallerLocation() {
        StackTraceElement[] stack = Thread.currentThread().getStackTrace();
        // stack[0] = getStackTrace, [1] = findCallerLocation, [2] = trace,
        // [3] = nextXxx (this class), [4+] = actual caller
        for (int i = 3; i < stack.length; i++) {
            String cls = stack[i].getClassName();
            if (cls.equals("org.eclipse.elk.graph.json.test.TracingRandom")
                    || cls.equals("java.util.Random")) {
                continue;
            }
            // Extract simple class name
            String simpleName = cls;
            int dot = cls.lastIndexOf('.');
            if (dot >= 0) {
                simpleName = cls.substring(dot + 1);
            }
            return simpleName + "::" + stack[i].getMethodName();
        }
        return "unknown";
    }
}
