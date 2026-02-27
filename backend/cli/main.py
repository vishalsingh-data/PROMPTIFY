import sys
import json

from firewall_engine.analyzer import PromptAnalyzer
from firewall_engine.logger import log_attack

def main():
    analyzer = PromptAnalyzer()

    json_mode = "--json" in sys.argv

    if not json_mode:
        print("\n=== LLM Guardian CLI ===")
        print("Type 'exit' to quit.\n")

    while True:
        user_input = input("Enter prompt to analyze: ")

        if user_input.lower() == "exit":
            print("Exiting...")
            sys.exit(0)
        elif user_input.lower() == "reset":
            analyzer.reset_conversation()
            print("Conversation memory reset.\n")
            continue

        result = analyzer.analyze(user_input)

        # Log risky activity
        if result["risk_score"] > 0:
            log_attack(user_input, result, source="user")

        # Display result
        if json_mode:
            print(json.dumps(result, indent=4))
        else:
            print("\n--- Analysis Result ---")
            print(f"Verdict: {result['verdict']}")
            print(f"Risk Score: {result['risk_score']}")
            print(f"Escalation Level: {result.get('escalation_level', 0)}")
            print(f"Categories Seen: {result.get('categories_seen', [])}")

            if result["findings"]:
                print("\nFindings:")
                for f in result["findings"]:
                    print(f"- [{f['category']}] {f['description']}")
            else:
                print("No suspicious patterns detected.")

            print("\n-----------------------\n")


if __name__ == "__main__":
    main()