Jekyll::Hooks.register [:posts, :pages], :pre_render do |doc|
    # (?m) ensures that ^ matches the start of each line.
    # ^(?!\s) asserts that the line does not start with any whitespace.
    doc.content = doc.content.gsub(/(?m)^(?!\s)!\[([^\]]*)\]\(([^)]+)\)/) do
      alt_text = Regexp.last_match(1)
      src      = Regexp.last_match(2)

      "{% include figure.liquid alt=\"#{alt_text}\" path=\"#{src}\" %}"
    end
  end
