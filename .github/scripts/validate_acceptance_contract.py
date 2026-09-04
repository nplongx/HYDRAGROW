import json
import sys

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 validate_acceptance_contract.py <path>")
        sys.exit(1)

    path = sys.argv[1]
    try:
        with open(path, 'r') as f:
            data = json.load(f)
        if 'requirement_id' not in data:
            raise ValueError("Missing requirement_id")
        if 'acceptance' not in data or not isinstance(data['acceptance'], list):
            raise ValueError("Missing or invalid acceptance array")
        print(f"Acceptance contract valid: {path}")
        print(f"Requirement: {data['requirement_id']}")
        print(f"Acceptance criteria: {len(data['acceptance'])}")
    except Exception as e:
        print(f"Validation failed: {e}")
        sys.exit(1)

if __name__ == '__main__':
    main()
