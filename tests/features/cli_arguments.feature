Feature: Detecting an explicit format flag in raw arguments
  Before full argument parsing, RustShot scans argv to learn whether the user
  passed the image format explicitly (--format / -f in any spelling). Values
  consumed by --output, -o, --screen and -s are skipped, so an output file
  named "-f" is never mistaken for a flag. The first argument (the program
  name) is always ignored.

  Scenario Outline: Recognising the spellings of the format flag
    Given the command line "<argv>"
    When the arguments are scanned for a format flag
    Then a format flag <verdict> detected

    Examples:
      | argv                             | verdict |
      | rustshot --format png            | is      |
      | rustshot -f png                  | is      |
      | rustshot --format=jpg            | is      |
      | rustshot -fjpg                   | is      |
      | rustshot -wfjpg                  | is      |
      | rustshot -wf jpg                 | is      |
      | rustshot -wofile.png             | is not  |
      | rustshot -wo -f                  | is not  |
      | rustshot -- -f                   | is not  |
      | rustshot --screen 0 --format png | is      |
      | rustshot --output shot.png       | is not  |
      | rustshot --output -f             | is not  |
      | rustshot -o -fpng                | is not  |
      | rustshot                         | is not  |
      | --format                         | is not  |
