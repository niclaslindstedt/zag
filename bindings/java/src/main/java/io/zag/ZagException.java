package io.zag;

/** Exception thrown when the zag process fails. */
public class ZagException extends Exception {

    private final Integer exitCode;
    private final String stderr;

    public ZagException(String message, Integer exitCode, String stderr) {
        super(message);
        this.exitCode = exitCode;
        this.stderr = stderr;
    }

    /** The process exit code, or null if the process could not be started. */
    public Integer exitCode() {
        return exitCode;
    }

    /** Captured stderr output from the process. */
    public String stderr() {
        return stderr;
    }
}
