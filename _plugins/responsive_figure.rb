Jekyll::Hooks.register [:posts, :pages], :pre_render do |doc|
    doc.content = doc.content.gsub(/(?mi)^(?!\s)!\[([^\]]*)\]\(([^)]+\.(?:jpg|jpeg|png|tiff|gif))\)(?:\r?\n)/) do
      alt_text = Regexp.last_match(1)
      src      = Regexp.last_match(2)

      replacement = "{% include figure.liquid alt=\"#{alt_text}\" path=\"#{src}\" %}"

      # Always append a newline.
      replacement + "\n"
    end
  end
