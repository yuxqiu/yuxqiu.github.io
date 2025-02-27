Jekyll::Hooks.register [:posts, :pages], :pre_render do |doc|
    doc.content = doc.content.gsub(/(?mi)^(\s*)!\[([^\]]*)\]\(([^)]+\.(?:jpg|jpeg|png|tiff|gif))\)(?:\r?\n)?/) do
      indent   = Regexp.last_match(1)
      alt_text = Regexp.last_match(2)
      src      = Regexp.last_match(3)

      replacement = "{% include figure.liquid alt=\"#{alt_text}\" path=\"#{src}\" indent=\"#{indent}\" %}"

      # Ensure a newline is always added after the replacement
      replacement + "\n"
    end
  end
