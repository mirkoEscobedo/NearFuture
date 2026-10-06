package nf.contract.canonical;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.text.Normalizer;
import java.util.Arrays;
import java.util.List;
import nf.contract.Unicode13;

/** Closed NF-CANON-1 conformance values. Counters preserve unsigned raw long bits. */
public final class Canonical {
    private Canonical() { }
    public record Counter(String key, long rawValue) { }
    public record Profile(byte[] optionalBytes, String optionalText, List<Long> integers, List<Counter> counters) {
        public Profile {
            optionalBytes = optionalBytes == null ? null : optionalBytes.clone();
            integers = List.copyOf(integers);
            counters = List.copyOf(counters);
        }
        @Override public byte[] optionalBytes() { return optionalBytes == null ? null : optionalBytes.clone(); }
    }
    public static byte[] encodeProfile(Profile profile) { return encodeProfile(profile, Limits.DEFAULT); }
    public static byte[] encodeProfile(Profile profile, Limits limits) {
        if (profile.integers.size() > limits.collectionItems() || profile.counters.size() > limits.collectionItems()
                || (long) profile.integers.size() + profile.counters.size() > limits.totalItems()) {
            throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        }
        byte[] bytes = profile.optionalBytes;
        if (bytes != null && bytes.length > limits.bytesBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        byte[] text = profile.optionalText == null ? null : utf8(profile.optionalText, limits.textBytes());
        long size = 27L + profile.integers.size() * 8L;
        if (bytes != null) size += 4L + bytes.length;
        if (text != null) size += 4L + text.length;
        byte[][] keys = new byte[profile.counters.size()][];
        byte[] previous = null;
        for (int i = 0; i < keys.length; i++) {
            byte[] key = utf8(profile.counters.get(i).key(), limits.textBytes());
            if (previous != null && Arrays.compareUnsigned(previous, key) >= 0) {
                throw new Validation(Arrays.equals(previous, key) ? Validation.Code.DUPLICATE_KEY : Validation.Code.KEY_ORDER);
            }
            previous = key;
            keys[i] = key;
            size += 12L + key.length;
            if (size > limits.documentBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        }
        if (size > limits.documentBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        Binary.Writer writer = new Binary.Writer((int) size);
        writer.header(255, 1);
        writer.number(bytes == null ? 0 : 1, 1);
        if (bytes != null) writer.field(bytes);
        writer.number(text == null ? 0 : 1, 1);
        if (text != null) writer.field(text);
        writer.number(profile.integers.size(), 4);
        for (long value : profile.integers) writer.number(value, 8);
        writer.number(profile.counters.size(), 4);
        for (int i = 0; i < keys.length; i++) {
            writer.field(keys[i]);
            writer.number(profile.counters.get(i).rawValue(), 8);
        }
        return writer.finish();
    }
    public static Profile decodeProfile(byte[] input) { return decodeProfile(input, Limits.DEFAULT); }
    public static Profile decodeProfile(byte[] input, Limits limits) { return decodeProfile(input, limits, limits.totalItems()); }
    static Profile decodeProfile(byte[] input, Limits limits, int remainingItems) {
        Binary.Reader reader = new Binary.Reader(input, limits, remainingItems);
        reader.header(255, 1);
        byte[] bytes = reader.presence() == 0 ? null : reader.field(limits.bytesBytes());
        byte[] textBytes = reader.presence() == 0 ? null : reader.field(limits.textBytes());
        String text = textBytes == null ? null : text(textBytes);
        if (textBytes != null) reader.allocate(textBytes.length * 2L);
        int integersCount = reader.entries(limits.collectionItems());
        reader.require(integersCount * 8);
        java.util.ArrayList<Long> integers = new java.util.ArrayList<>(integersCount);
        for (int i = 0; i < integersCount; i++) integers.add(reader.number(8));
        int countersCount = reader.entries(limits.collectionItems());
        java.util.ArrayList<Counter> counters = new java.util.ArrayList<>(countersCount);
        byte[] previous = null;
        for (int i = 0; i < countersCount; i++) {
            byte[] key = reader.field(limits.textBytes());
            if (previous != null && Arrays.compareUnsigned(previous, key) >= 0) {
                throw new Validation(Arrays.equals(previous, key) ? Validation.Code.DUPLICATE_KEY : Validation.Code.KEY_ORDER);
            }
            previous = key;
            reader.allocate(key.length * 2L);
            counters.add(new Counter(text(key), reader.number(8)));
        }
        reader.finish();
        return new Profile(bytes, text, integers, counters);
    }
    public static byte[] validateText(String text, int maximum) { return utf8(text, maximum); }
    static byte[] utf8(String text, int maximum) {
        if (maximum < 1 || maximum > Limits.DEFAULT.textBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        if (text.length() > maximum) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        for (int i = 0; i < text.length(); i++) {
            char value = text.charAt(i);
            int scalar = value;
            if (Character.isHighSurrogate(value)) {
                if (++i >= text.length() || !Character.isLowSurrogate(text.charAt(i))) throw new Validation(Validation.Code.INVALID_UTF8);
                scalar = Character.toCodePoint(value, text.charAt(i));
            } else if (Character.isLowSurrogate(value)) throw new Validation(Validation.Code.INVALID_UTF8);
            if (!Unicode13.contains(scalar)) throw new Validation(Validation.Code.INVALID_UTF8);
        }
        if (!Normalizer.isNormalized(text, Normalizer.Form.NFC)) throw new Validation(Validation.Code.NON_NFC);
        byte[] bytes = text.getBytes(StandardCharsets.UTF_8);
        if (bytes.length > maximum) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        return bytes;
    }
    static String text(byte[] bytes) {
        String value = new String(bytes, StandardCharsets.UTF_8);
        if (!Arrays.equals(bytes, value.getBytes(StandardCharsets.UTF_8))) throw new Validation(Validation.Code.INVALID_UTF8);
        utf8(value, Limits.DEFAULT.textBytes());
        return value;
    }
    public static byte[] sha256(byte[] canonicalBytes) {
        if (canonicalBytes.length > Limits.DEFAULT.documentBytes()) throw new Validation(Validation.Code.LIMIT_EXCEEDED);
        try { return MessageDigest.getInstance("SHA-256").digest(canonicalBytes); }
        catch (NoSuchAlgorithmException impossible) { throw new IllegalStateException("SHA-256 unavailable"); }
    }
}
