def first_divergence(log_a_path, log_b_path, offset=0):
    """
    offset: how many lines to skip in log_b before comparing.
    Positive offset = log_b is "ahead" of log_a by that many lines.
    Negative offset = log_a is ahead of log_b.
    """
    with open(log_a_path) as fa, open(log_b_path) as fb:
        lines_a = fa.readlines()
        lines_b = fb.readlines()

    if offset > 0:
        lines_b = lines_b[offset:]
    elif offset < 0:
        lines_a = lines_a[-offset:]

    for i, (a, b) in enumerate(zip(lines_a, lines_b), start=1):
        if a.strip() != b.strip():
            print(f"Divergence at instruction {i} (after offset applied):")
            print(f"  yours:    {a.strip()}")
            print(f"  sameboy:  {b.strip()}")
            return i

    print("No divergence found in the overlapping range.")
    return None

# Try offset=1 first since that's the usual case
first_divergence("../mine.log", "trace.log", offset=0)