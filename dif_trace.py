import sys

def parse_fields(line):
    """Split a trace line into a dict of field:value pairs for comparison/display."""
    fields = {}
    for token in line.strip().split():
        if ':' in token:
            key, _, val = token.partition(':')
            fields[key] = val
    return fields

def diff_fields(a_fields, b_fields):
    """Return list of field names that differ between the two parsed lines."""
    keys = set(a_fields) | set(b_fields)
    return [k for k in keys if a_fields.get(k) != b_fields.get(k)]

def first_divergence(log_a_path, log_b_path, offset=0, interactive=True):
    with open(log_a_path) as fa, open(log_b_path) as fb:
        lines_a = fa.readlines()
        lines_b = fb.readlines()

    if offset > 0:
        lines_b = lines_b[offset:]
    elif offset < 0:
        lines_a = lines_a[-offset:]

    skipped = 0
    i = 0
    for a, b in zip(lines_a, lines_b):
        i += 1
        a_s, b_s = a.strip(), b.strip()
        if a_s == b_s:
            continue

        a_fields = parse_fields(a_s)
        b_fields = parse_fields(b_s)
        changed = diff_fields(a_fields, b_fields)

        print(f"\nDivergence at instruction {i} (after offset applied):")
        print(f"  yours:    {a_s}")
        print(f"  sameboy:  {b_s}")
        print(f"  differing fields: {', '.join(changed)}")

        if not interactive:
            return i

        choice = input("  [c]ontinue past this, [s]top here, [a]uto-continue for rest: ").strip().lower()
        if choice == 'c':
            skipped += 1
            continue
        elif choice == 'a':
            skipped += 1
            interactive = False  # stop asking, just log and keep going
            continue
        else:  # 's' or anything else
            print(f"\nStopped. Total divergences skipped before this one: {skipped}")
            return i

    print(f"\nNo further divergence found in the overlapping range. Total skipped: {skipped}")
    return None

if __name__ == "__main__":
    log_a = sys.argv[1]
    log_b = sys.argv[2]
    offset = int(sys.argv[3]) if len(sys.argv) > 3 else 0
    first_divergence(log_a, log_b, offset=offset)